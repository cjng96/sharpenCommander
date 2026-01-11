use std::cmp::min;
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Stdout};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;

use crate::app::{open_in_editor, AppContext};
use crate::config::RegItem;
use crate::git;
use crate::system::{app_log, system, system_logged, system_safe, system_stream};
use crate::util::{match_disorder, match_disorder_count, walk_dirs};

const INPUT_PREFIX: &str = "$ ";
static REDRAW_REQUEST: AtomicBool = AtomicBool::new(false);

pub fn run(ctx: &mut AppContext) -> anyhow::Result<()> {
    let mut app = App::new(ctx)?;
    run_app(&mut app)
}

pub fn run_git_status(ctx: &mut AppContext) -> anyhow::Result<()> {
    let state = GitStatusState::new(ctx)?;
    let mut app = App::new(ctx)?;
    app.screen = Screen::GitStatus(state);
    run_app(&mut app)
}

pub fn run_find(ctx: &mut AppContext, args: &[String]) -> anyhow::Result<()> {
    let state = FindState::from_args(ctx, args)?;
    let mut app = App::new(ctx)?;
    app.screen = Screen::Find(state);
    run_app(&mut app)
}

pub fn run_grep(ctx: &mut AppContext, args: &[String]) -> anyhow::Result<()> {
    let state = GrepState::from_args(ctx, args)?;
    let mut app = App::new(ctx)?;
    app.screen = Screen::Grep(state);
    run_app(&mut app)
}

pub fn git_push(_ctx: &mut AppContext) -> anyhow::Result<()> {
    let current = git::get_current_branch()?;
    let tracking = git::get_tracking_branch();
    let mut targets = vec![current.clone()];
    if let Some(tracking) = tracking.clone() {
        targets.push(tracking.split('/').last().unwrap_or("").to_string());
        let gap = git::commit_gap(&current, &tracking)?;
        if gap == 0 {
            git::print_status()?;
            return Err(anyhow::anyhow!("There is no commit to push"));
        }
        println!("There are {gap} commits to push");
        println!("{}", git::commit_log_between(&current, &tracking)?);
    }

    println!("Input remote branch name you push to:");
    let target = input_prompt_with_list(&targets)?;
    if target.is_empty() {
        return Err(anyhow::anyhow!("Push is canceled"));
    }
    let remote = if let Some(tracking) = tracking {
        tracking.split('/').next().unwrap_or("origin").to_string()
    } else {
        let remotes = git::remote_list()?;
        println!("Input remote name to push to:");
        input_prompt_with_list(&remotes)?
    };
    let cmd = format!("git push {} {}:{}", remote, current, target);
    let (out, code) = system_safe(&cmd);
    println!("{out}");
    if code != 0 {
        println!("Push failed.");
    }
    Ok(())
}

pub fn git_fetch(ctx: &mut AppContext, target: Option<&str>) -> anyhow::Result<()> {
    git_action(ctx, target, GitAction::Fetch)
}

pub fn git_merge(ctx: &mut AppContext, target: Option<&str>) -> anyhow::Result<()> {
    git_action(ctx, target, GitAction::Merge)
}

pub fn git_update(ctx: &mut AppContext, target: Option<&str>) -> anyhow::Result<()> {
    git_action(ctx, target, GitAction::Update)
}

pub fn git_status_component(ctx: &mut AppContext, target: &str) -> anyhow::Result<()> {
    let mut actor = GitActor::new(ctx);
    actor.action(GitAction::Status, if target.is_empty() { None } else { Some(target) })?;
    Ok(())
}

fn git_action(ctx: &mut AppContext, target: Option<&str>, action: GitAction) -> anyhow::Result<()> {
    let mut actor = GitActor::new(ctx);
    actor.action(action, target)?;
    Ok(())
}

fn run_app(app: &mut App<'_>) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = app.run(&mut terminal);
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res
}

fn input_prompt_with_list(items: &[String]) -> anyhow::Result<String> {
    use std::io::Write;
    let mut input = String::new();
    if !items.is_empty() {
        println!("Choices: {}", items.join(", "));
    }
    print!("> ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

struct App<'a> {
    ctx: &'a mut AppContext,
    screen: Screen,
    toast: Option<(String, Instant)>,
}

impl<'a> App<'a> {
    fn new(ctx: &'a mut AppContext) -> anyhow::Result<Self> {
        let main = MainState::new(ctx)?;
        Ok(Self {
            ctx,
            screen: Screen::Main(main),
            toast: None,
        })
    }

    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();
        loop {
            if REDRAW_REQUEST.swap(false, Ordering::SeqCst) {
                terminal.clear()?;
            }
            terminal.draw(|f| self.render(f))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        if self.on_key(key)? {
                            return Ok(());
                        }
                    }
                    Event::Mouse(me) => {
                        if self.on_mouse(me)? {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
            if last_tick.elapsed() >= tick_rate {
                if let Some((_, expire)) = self.toast {
                    if Instant::now() > expire {
                        self.toast = None;
                    }
                }
                last_tick = Instant::now();
            }
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        match &mut self.screen {
            Screen::Main(state) => state.render(f),
            Screen::Find(state) => state.render(f),
            Screen::Grep(state) => state.render(f),
            Screen::GitStatus(state) => state.render(f),
            Screen::GitCommit(state) => state.render(f),
            Screen::RegList(state) => state.render(f),
            Screen::Goto(state) => state.render(f),
        }

        if let Some((msg, _)) = &self.toast {
            let area = f.size();
            let toast_height = 3;
            let toast_area = Rect {
                x: area.x + area.width / 4,
                y: area.y + area.height.saturating_sub(toast_height + 2),
                width: area.width / 2,
                height: toast_height,
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let p = Paragraph::new(format!("{}", msg))
                .block(block)
                .style(Style::default().fg(Color::Gray))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(Clear, toast_area);
            f.render_widget(p, toast_area);
        }
    }

    fn set_toast(&mut self, msg: &str, duration: Duration) {
        self.toast = Some((msg.to_string(), Instant::now() + duration));
    }

    fn on_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }
        let action = match &mut self.screen {
            Screen::Main(state) => state.on_key(self.ctx, key)?,
            Screen::Find(state) => state.on_key(self.ctx, key)?,
            Screen::Grep(state) => state.on_key(self.ctx, key)?,
            Screen::GitStatus(state) => state.on_key(self.ctx, key)?,
            Screen::GitCommit(state) => state.on_key(self.ctx, key)?,
            Screen::RegList(state) => state.on_key(self.ctx, key)?,
            Screen::Goto(state) => state.on_key(self.ctx, key)?,
        };
        match action {
            Action::None => Ok(false),
            Action::Exit => Ok(true),
            Action::Switch(screen) => {
                self.screen = screen;
                Ok(false)
            }
            Action::Toast(msg) => {
                self.set_toast(&msg, Duration::from_secs(2));
                Ok(false)
            }
        }
    }

    fn on_mouse(&mut self, me: MouseEvent) -> anyhow::Result<bool> {
        let action = match &mut self.screen {
            Screen::Main(state) => state.on_mouse(self.ctx, me)?,
            Screen::Find(state) => state.on_mouse(self.ctx, me)?,
            Screen::Grep(state) => state.on_mouse(self.ctx, me)?,
            Screen::GitStatus(state) => state.on_mouse(self.ctx, me)?,
            Screen::GitCommit(state) => state.on_mouse(self.ctx, me)?,
            Screen::RegList(state) => state.on_mouse(self.ctx, me)?,
            Screen::Goto(state) => state.on_mouse(self.ctx, me)?,
        };
        match action {
            Action::None => Ok(false),
            Action::Exit => Ok(true),
            Action::Switch(screen) => {
                self.screen = screen;
                Ok(false)
            }
            Action::Toast(msg) => {
                self.set_toast(&msg, Duration::from_secs(2));
                Ok(false)
            }
        }
    }
}

enum Screen {
    Main(MainState),
    Find(FindState),
    Grep(GrepState),
    GitStatus(GitStatusState),
    GitCommit(GitCommitState),
    RegList(RegListState),
    Goto(GotoState),
}

enum Action {
    None,
    Exit,
    Switch(Screen),
    Toast(String),
}

struct MainState {
    cwd: PathBuf,
    items: Vec<DirEntry>,
    list_state: ListState,
    input: String,
    input_mode: bool,
    cmd_list: Vec<String>,
    work_list: Vec<PathBuf>,
    work_idx: usize,
    list_area: Option<Rect>,
    last_click: Option<(Instant, usize)>,
}

struct DirEntry {
    name: String,
    is_dir: bool,
}

impl MainState {
    fn new(_ctx: &mut AppContext) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let mut state = Self {
            cwd,
            items: Vec::new(),
            list_state: ListState::default(),
            input: String::new(),
            input_mode: false,
            cmd_list: Vec::new(),
            work_list: vec![std::env::current_dir()?],
            work_idx: 0,
            list_area: None,
            last_click: None,
        };
        state.refresh();
        if !state.items.is_empty() {
            state.list_state.select(Some(0));
        }
        Ok(state)
    }

    fn refresh(&mut self) {
        let mut list = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.cwd) {
            for entry in rd.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == ".dcdata" {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    list.push(DirEntry {
                        name: name.to_string(),
                        is_dir,
                    });
                }
            }
        }
        list.sort_by_key(|e| e.name.clone());
        self.items = list;
        self.items.insert(
            0,
            DirEntry {
                name: "..".to_string(),
                is_dir: true,
            },
        );
        if self.items.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().unwrap_or(0) >= self.items.len() {
            self.list_state.select(Some(0));
        }
    }

    fn focus_name(&self) -> Option<String> {
        let idx = self.list_state.selected()?;
        self.items.get(idx).map(|e| e.name.clone())
    }

    fn enter_dir(&mut self, name: &str) {
        if name == ".." {
            if let Some(parent) = self.cwd.parent() {
                if std::env::set_current_dir(parent).is_ok() {
                    self.cwd = parent.to_path_buf();
                    self.refresh();
                    self.list_state.select(Some(0));
                }
            }
            return;
        }
        let target = self.cwd.join(name);
        if target.is_dir() {
            if std::env::set_current_dir(&target).is_ok() {
                self.cwd = target;
                self.refresh();
                self.list_state.select(Some(0));
            }
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let cwd_str = self.cwd.to_string_lossy();
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
            .split(f.size());

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(layout[0]);

        let header = format!(">> sc V{} - {}", env!("CARGO_PKG_VERSION"), cwd_str);
        let list_block = Block::default().title(header);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| {
                let style = if i.is_dir {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new(i.name.as_str()).style(style)
            })
            .collect();
        let list = List::new(items)
            .block(list_block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, left[0], &mut self.list_state);
        self.list_area = Some(left[0]);

        let cmd_items = if self.cmd_list.is_empty() {
            vec![ListItem::new("< Nothing >")]
        } else {
            self.cmd_list
                .iter()
                .map(|i| ListItem::new(i.as_str()))
                .collect()
        };
        let cmd_list = List::new(cmd_items).block(Block::default().title("Cmd"));
        f.render_widget(cmd_list, left[1]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[1]);

        let work_items: Vec<ListItem> = self
            .work_list
            .iter()
            .map(|p| ListItem::new(p.to_string_lossy().to_string()))
            .collect();
        let work_list = List::new(work_items).block(Block::default().title("Workspace"));
        let mut work_state = ListState::default();
        work_state.select(Some(self.work_idx));
        f.render_stateful_widget(work_list, right[0], &mut work_state);

        let input_title = if self.input_mode { "Input" } else { "Idle" };
        let input = Paragraph::new(format!("{}{}", INPUT_PREFIX, self.input))
            .block(Block::default().title(input_title));
        f.render_widget(input, right[1]);
    }

    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                    self.input.clear();
                }
                KeyCode::Enter => {
                    let cmd = self.input.trim().to_string();
                    self.input_mode = false;
                    self.input.clear();
                    return self.run_command(ctx, &cmd);
                }
                KeyCode::Char(c) => self.input.push(c),
                KeyCode::Backspace => {
                    self.input.pop();
                }
                _ => {}
            }
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Char('q') => return Ok(Action::Exit),
            KeyCode::Down => self.select_next(),
            KeyCode::Up => self.select_prev(),
            KeyCode::Char('j') if key.modifiers.is_empty() => self.select_next(),
            KeyCode::Char('k') if key.modifiers.is_empty() => self.select_prev(),
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::ALT) => {
                if let Some(parent) = self.cwd.parent() {
                    if std::env::set_current_dir(parent).is_ok() {
                        self.cwd = parent.to_path_buf();
                        self.refresh();
                        self.list_state.select(Some(0));
                    }
                }
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::ALT) => {
                if let Some(name) = self.focus_name() {
                    self.enter_dir(&name);
                }
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::ALT) => self.select_next(),
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::ALT) => self.select_prev(),
            KeyCode::Char('J') => self.select_step(10),
            KeyCode::Char('K') => self.select_step(-10),
            KeyCode::Char('u') | KeyCode::Char('.') | KeyCode::Char('U') => {
                if let Some(parent) = self.cwd.parent() {
                    if std::env::set_current_dir(parent).is_ok() {
                        self.cwd = parent.to_path_buf();
                        self.refresh();
                        self.list_state.select(Some(0));
                    }
                }
            }
            KeyCode::Char('h') | KeyCode::Enter => {
                if let Some(name) = self.focus_name() {
                    self.enter_dir(&name);
                }
            }
            KeyCode::Char('E') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(entry) = self.items.get(idx) {
                        let path = self.cwd.join(&entry.name);
                        if path.is_file() {
                            open_in_editor(&ctx.config.edit_app, path.to_string_lossy().as_ref());
                        }
                    }
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = true;
            }
            KeyCode::Char('L') => {
                return Ok(Action::Switch(Screen::RegList(RegListState::new(ctx)?)));
            }
            KeyCode::Char('C') => {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("<unknown>"));
                app_log(&format!(
                    "Key C (Main) cwd={}",
                    cwd.to_string_lossy()
                ));
                match GitStatusState::new(ctx) {
                    Ok(state) => return Ok(Action::Switch(Screen::GitStatus(state))),
                    Err(err) => {
                        app_log(&format!("Key C (Main) error: {}", err));
                        return Ok(Action::Toast(err.to_string()));
                    }
                }
            }
            KeyCode::Char('g') => {
                return Ok(Action::Switch(Screen::Goto(GotoState::new(ctx)?)));
            }
            KeyCode::Char('R') => {
                if let Some(name) = self.focus_name() {
                    let path = self.cwd.join(&name);
                    if path.is_dir() {
                        let path_str = path.to_string_lossy().to_string();
                        if ctx.reg_find_by_path(&path_str).is_none() {
                            ctx.reg_add(&path_str)?;
                        }
                    }
                }
            }
            KeyCode::Char('F') => {
                if let Some(name) = self.focus_name() {
                    let path = self.cwd.join(&name);
                    if path.is_dir() {
                        let target = path.to_string_lossy().to_string();
                        with_terminal_pause(|| {
                            let old = std::env::current_dir()?;
                            if std::env::set_current_dir(&target).is_ok() {
                                println!("$ git pull -r");
                                let _ = system_stream("git pull -r");
                                let _ = std::env::set_current_dir(old);
                            }
                            Ok(())
                        })?;
                    }
                }
            }
            KeyCode::Char('T') => {
                if let Some(name) = self.focus_name() {
                    let path = self.cwd.join(&name);
                    let target = if path.is_dir() {
                        path
                    } else {
                        self.cwd.clone()
                    };
                    with_terminal_pause(|| {
                        let old = std::env::current_dir()?;
                        if std::env::set_current_dir(&target).is_ok() {
                            let _ = system_stream("tig");
                            let _ = std::env::set_current_dir(old);
                        }
                        Ok(())
                    })?;
                }
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn run_command(&mut self, ctx: &mut AppContext, cmd: &str) -> anyhow::Result<Action> {
        if cmd.is_empty() {
            return Ok(Action::None);
        }
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts[0] {
            "find" | "ff" => {
                let args: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
                return Ok(Action::Switch(Screen::Find(FindState::from_args(
                    ctx, &args,
                )?)));
            }
            "grep" | "gg" => {
                let args: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
                return Ok(Action::Switch(Screen::Grep(GrepState::from_args(
                    ctx, &args,
                )?)));
            }
            "reg" => {
                let path = self.cwd.to_string_lossy().to_string();
                ctx.reg_add(&path)?;
            }
            "st" => {
                let target = parts.get(1).map(|s| *s);
                let mut actor = GitActor::new(ctx);
                actor.action(GitAction::Status, target)?;
            }
            "update" => {
                let target = parts.get(1).map(|s| *s);
                let mut actor = GitActor::new(ctx);
                actor.action(GitAction::Update, target)?;
            }
            "fetch" => {
                let target = parts.get(1).map(|s| *s);
                let mut actor = GitActor::new(ctx);
                actor.action(GitAction::Fetch, target)?;
            }
            _ => {
                self.cmd_list = vec![format!("Unknown command: {}", cmd)];
            }
        }
        Ok(Action::None)
    }

    fn select_next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => min(i + 1, self.items.len().saturating_sub(1)),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_step(&mut self, step: i32) {
        let cur = self.list_state.selected().unwrap_or(0) as i32;
        let next = (cur + step).clamp(0, self.items.len().saturating_sub(1) as i32);
        self.list_state.select(Some(next as usize));
    }

    fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.items.len() {
                                self.list_state.select(Some(idx));
                                if matches!(me.kind, MouseEventKind::Down(_)) {
                                    if is_double_click(&mut self.last_click, idx) {
                                        if let Some(name) = self.focus_name() {
                                            self.enter_dir(&name);
                                        }
                                    }
                                }
                            }
                        }
                        if matches!(me.kind, MouseEventKind::ScrollDown) {
                            self.select_next();
                        } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                            self.select_prev();
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(Action::None)
    }
}

struct FindState {
    files: Vec<String>,
    list_state: ListState,
    content: Vec<String>,
    list_area: Option<Rect>,
    content_area: Option<Rect>,
    content_scroll: u16,
    last_click: Option<(Instant, usize)>,
}

impl FindState {
    fn from_args(_ctx: &mut AppContext, args: &[String]) -> anyhow::Result<Self> {
        let cmd = if args.len() > 1 {
            let rest = args[1..].join(" ");
            if rest.trim_start().starts_with('-') {
                format!("find . {}", rest)
            } else {
                format!("find {}", rest)
            }
        } else {
            "find .".to_string()
        };
        let (out, _code) = system_safe(&cmd);
        let mut files: Vec<String> = out.lines().map(|l| l.to_string()).collect();
        files.retain(|s| !s.trim().is_empty());
        let mut state = Self {
            files,
            list_state: ListState::default(),
            content: vec!["< Nothing to display >".to_string()],
            list_area: None,
            content_area: None,
            content_scroll: 0,
            last_click: None,
        };
        state.list_state.select(Some(0));
        state.load_content();
        Ok(state)
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(f.size());
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();
        let list = List::new(items)
            .block(Block::default().title("Find"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, layout[0], &mut self.list_state);
        self.list_area = Some(layout[0]);

        let text = Text::from(self.content.join("\n"));
        let view = Paragraph::new(text)
            .block(Block::default().title("Content"));
        f.render_widget(view.scroll((self.content_scroll, 0)), layout[1]);
        self.content_area = Some(layout[1]);
    }

    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
            }
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Enter => {
                if let Some(file) = self.focus_file() {
                    let path = Path::new(&file);
                    if let Some(parent) = path.parent() {
                        ctx.save_path(parent.to_string_lossy().as_ref())?;
                    }
                }
            }
            KeyCode::Char('E') => {
                if let Some(file) = self.focus_file() {
                    open_in_editor(&ctx.config.edit_app, &file);
                }
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn focus_file(&self) -> Option<String> {
        let idx = self.list_state.selected()?;
        self.files.get(idx).cloned()
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => min(i + 1, self.files.len().saturating_sub(1)),
            None => 0,
        };
        self.list_state.select(Some(i));
        self.load_content();
    }

    fn prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
        self.load_content();
    }

    fn load_content(&mut self) {
        if let Some(file) = self.focus_file() {
            let text = std::fs::read_to_string(&file)
                .unwrap_or_else(|_| format!("No utf8 file[size:{}]", file_size(&file)));
            self.content = text.replace('\t', "    ").lines().map(|s| s.to_string()).collect();
        }
    }

    fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.files.len() {
                                self.list_state.select(Some(idx));
                                self.load_content();
                                if matches!(me.kind, MouseEventKind::Down(_)) {
                                    if is_double_click(&mut self.last_click, idx) {
                                        if let Some(file) = self.focus_file() {
                                            let path = Path::new(&file);
                                            if let Some(parent) = path.parent() {
                                                let _ = parent.to_string_lossy();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if matches!(me.kind, MouseEventKind::ScrollDown) {
                            self.next();
                        } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                            self.prev();
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(area) = self.content_area {
            if area.contains(mouse_pos(&me)) {
                if matches!(me.kind, MouseEventKind::ScrollDown) {
                    self.content_scroll = self.content_scroll.saturating_add(3);
                } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                    self.content_scroll = self.content_scroll.saturating_sub(3);
                }
            }
        }
        Ok(Action::None)
    }
}

struct GrepState {
    lines: Vec<String>,
    list_state: ListState,
    list_area: Option<Rect>,
    last_click: Option<(Instant, usize)>,
}

impl GrepState {
    fn from_args(ctx: &mut AppContext, args: &[String]) -> anyhow::Result<Self> {
        let cmd = if args.len() > 1 {
            let rest = args[1..].join(" ");
            format!("{} --group --color {}", ctx.config.grep_app, rest)
        } else {
            format!("{} --group --color", ctx.config.grep_app)
        };
        let (out, _code) = system_safe(&cmd);
        let clean = strip_ansi(out);
        let mut lines: Vec<String> = clean.lines().map(|l| l.to_string()).collect();
        if lines.is_empty() {
            lines.push("< No result >".to_string());
        }
        let mut state = Self {
            lines,
            list_state: ListState::default(),
            list_area: None,
            last_click: None,
        };
        state.list_state.select(Some(0));
        Ok(state)
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let items: Vec<ListItem> = self
            .lines
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();
        let list = List::new(items)
            .block(Block::default().title("Grep"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, f.size(), &mut self.list_state);
        self.list_area = Some(f.size());
    }

    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
            }
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Enter => {
                if let Some(line) = self.focus_line() {
                    if !line.contains(':') {
                        return Ok(Action::None);
                    }
                    let file = line.split(':').next().unwrap_or("");
                    let path = Path::new(file);
                    if let Some(parent) = path.parent() {
                        ctx.save_path(parent.to_string_lossy().as_ref())?;
                    }
                }
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn focus_line(&self) -> Option<String> {
        let idx = self.list_state.selected()?;
        self.lines.get(idx).cloned()
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => min(i + 1, self.lines.len().saturating_sub(1)),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.lines.len() {
                                self.list_state.select(Some(idx));
                                if matches!(me.kind, MouseEventKind::Down(_)) {
                                    if is_double_click(&mut self.last_click, idx) {
                                        if let Some(line) = self.focus_line() {
                                            if !line.contains(':') {
                                                return Ok(Action::None);
                                            }
                                            let file = line.split(':').next().unwrap_or("");
                                            let path = Path::new(file);
                                            if let Some(parent) = path.parent() {
                                                let _ = parent.to_string_lossy();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if matches!(me.kind, MouseEventKind::ScrollDown) {
                            self.next();
                        } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                            self.prev();
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(Action::None)
    }
}

struct GitStatusState {
    items: Vec<GitItem>,
    list_state: ListState,
    content: Vec<String>,
    list_area: Option<Rect>,
    content_area: Option<Rect>,
    content_scroll: u16,
    last_click: Option<(Instant, usize)>,
}

impl GitStatusState {
    fn new(_ctx: &mut AppContext) -> anyhow::Result<Self> {
        let items = build_git_items()?;
        if items.iter().all(|i| i.kind != GitItemKind::Entry) {
            return Err(anyhow::anyhow!("No modified or untracked files"));
        }
        let mut state = Self {
            items,
            list_state: ListState::default(),
            content: vec!["< Nothing to display >".to_string()],
            list_area: None,
            content_area: None,
            content_scroll: 0,
            last_click: None,
        };
        state.list_state.select(state.first_selectable());
        state.load_content()?;
        Ok(state)
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(f.size());
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let style = match item.kind {
                    GitItemKind::Header => Style::default().fg(Color::DarkGray),
                    GitItemKind::Entry => Style::default(),
                };
                ListItem::new(item.label.as_str()).style(style)
            })
            .collect();
        let list = List::new(items)
            .block(Block::default().title("Git status"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, layout[0], &mut self.list_state);
        self.list_area = Some(layout[0]);

        let diff_lines = format_diff_lines(&self.content, layout[1].width);
        let text = Text::from(diff_lines);
        let view = Paragraph::new(text)
            .block(Block::default().title("Diff"));
        f.render_widget(view.scroll((self.content_scroll, 0)), layout[1]);
        self.content_area = Some(layout[1]);
    }

    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
            }
            KeyCode::Char('j') | KeyCode::Down => self.next()?,
            KeyCode::Char('k') | KeyCode::Up => self.prev()?,
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if let Some(name) = self.focus_file_name() {
                    system(&format!("git add \"{}\"", name))?;
                    self.items = build_git_items()?;
                    self.list_state.select(self.first_selectable());
                    self.load_content()?;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Some(name) = self.focus_file_name() {
                    system(&format!("git reset \"{}\"", name))?;
                    self.items = build_git_items()?;
                    self.list_state.select(self.first_selectable());
                    self.load_content()?;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(name) = self.focus_file_name() {
                    system(&format!("git checkout -- \"{}\"", name))?;
                    self.items = build_git_items()?;
                    self.list_state.select(self.first_selectable());
                    self.load_content()?;
                }
            }
            KeyCode::Char('C') => {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("<unknown>"));
                app_log(&format!(
                    "Key C (GitStatus) cwd={}",
                    cwd.to_string_lossy()
                ));
                match GitCommitState::new(ctx) {
                    Ok(state) => return Ok(Action::Switch(Screen::GitCommit(state))),
                    Err(err) => {
                        app_log(&format!("Key C (GitStatus) error: {}", err));
                        return Ok(Action::Toast(err.to_string()));
                    }
                }
            }
            KeyCode::Char('E') => {
                if let Some(name) = self.focus_file_name() {
                    open_in_editor(&ctx.config.edit_app, &name);
                }
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn focus_file_name(&self) -> Option<String> {
        let idx = self.list_state.selected()?;
        let item = self.items.get(idx)?;
        if item.kind != GitItemKind::Entry {
            return None;
        }
        item.path.clone()
    }

    fn next(&mut self) -> anyhow::Result<()> {
        let next = self.next_selectable(1);
        if let Some(i) = next {
            self.list_state.select(Some(i));
            self.load_content()?;
        }
        Ok(())
    }

    fn prev(&mut self) -> anyhow::Result<()> {
        let prev = self.prev_selectable(1);
        if let Some(i) = prev {
            self.list_state.select(Some(i));
            self.load_content()?;
        }
        Ok(())
    }

    fn load_content(&mut self) -> anyhow::Result<()> {
        app_log(&format!(
            "GitStatusState::load_content focus_file_name={:?}",
            self.focus_file_name()
        ));
        if let Some(name) = self.focus_file_name() {
            let status = self
                .list_state
                .selected()
                .and_then(|i| self.items.get(i))
                .and_then(|x| x.status.clone())
                .unwrap_or_default();
            let out = if Path::new(&name).is_dir() {
                format!("{} is folder", name)
            } else if status == "?" {
                std::fs::read_to_string(&name)
                    .unwrap_or_else(|_| format!("No utf8 file[size:{}]", file_size(&name)))
            } else if status == "s" {
                system(&format!("git diff --color --staged \"{}\"", name))?
            } else {
                system(&format!("git diff --color \"{}\"", name))?
            };
            self.content = strip_ansi(out).replace('\t', "    ").lines().map(|s| s.to_string()).collect();
        }
        Ok(())
    }

    fn first_selectable(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.kind == GitItemKind::Entry)
    }

    fn next_selectable(&self, step: usize) -> Option<usize> {
        let start = self.list_state.selected().unwrap_or(0);
        let mut idx = start + step;
        while idx < self.items.len() {
            if self.items[idx].kind == GitItemKind::Entry {
                return Some(idx);
            }
            idx += 1;
        }
        None
    }

    fn prev_selectable(&self, step: usize) -> Option<usize> {
        let mut idx = self.list_state.selected().unwrap_or(0);
        for _ in 0..=step {
            if idx == 0 {
                break;
            }
            idx -= 1;
            if self.items[idx].kind == GitItemKind::Entry {
                return Some(idx);
            }
        }
        None
    }

    fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.items.len() {
                                if self.items[idx].kind == GitItemKind::Entry {
                                    self.list_state.select(Some(idx));
                                    let _ = self.load_content();
                                    if matches!(me.kind, MouseEventKind::Down(_)) {
                                        let _ = is_double_click(&mut self.last_click, idx);
                                    }
                                }
                            }
                        }
                        if matches!(me.kind, MouseEventKind::ScrollDown) {
                            let _ = self.next();
                        } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                            let _ = self.prev();
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(area) = self.content_area {
            if area.contains(mouse_pos(&me)) {
                if matches!(me.kind, MouseEventKind::ScrollDown) {
                    self.content_scroll = self.content_scroll.saturating_add(3);
                } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                    self.content_scroll = self.content_scroll.saturating_sub(3);
                }
            }
        }
        Ok(Action::None)
    }
}

#[derive(Clone, PartialEq)]
enum GitItemKind {
    Header,
    Entry,
}

struct GitItem {
    label: String,
    status: Option<String>,
    kind: GitItemKind,
    path: Option<String>,
}

fn build_git_items() -> anyhow::Result<Vec<GitItem>> {
    let list = git::status_file_list()?;
    app_log(&format!(
        "build_git_items list={:?}",
        list
    ));
    let mut modified = Vec::new();
    let mut untracked = Vec::new();
    let mut staged = Vec::new();
    for (line, status_code) in list {
        if status_code == "??" {
            untracked.push((line, "?".to_string()));
            continue;
        }
        let bytes = status_code.as_bytes();
        let staged_flag = bytes.get(0).copied().unwrap_or(b' ') != b' ';
        let modified_flag = bytes.get(1).copied().unwrap_or(b' ') != b' ';
        if staged_flag {
            staged.push((line.clone(), "s".to_string()));
        }
        if modified_flag {
            modified.push((line, "".to_string()));
        }
    }
    let mut items = Vec::new();
    if !modified.is_empty() {
        items.push(GitItem {
            label: "< Modified >".to_string(),
            status: None,
            kind: GitItemKind::Header,
            path: None,
        });
        for (clean, status) in modified {
            items.push(GitItem {
                label: clean.clone(),
                status: Some(status),
                kind: GitItemKind::Entry,
                path: git_file_last_name(&clean),
            });
        }
    }
    if !untracked.is_empty() {
        items.push(GitItem {
            label: "< Untracked >".to_string(),
            status: None,
            kind: GitItemKind::Header,
            path: None,
        });
        for (clean, status) in untracked {
            items.push(GitItem {
                label: clean.clone(),
                status: Some(status),
                kind: GitItemKind::Entry,
                path: git_file_last_name(&clean),
            });
        }
    }
    if !staged.is_empty() {
        items.push(GitItem {
            label: "< Staged >".to_string(),
            status: None,
            kind: GitItemKind::Header,
            path: None,
        });
        for (clean, status) in staged {
            items.push(GitItem {
                label: clean.clone(),
                status: Some(status),
                kind: GitItemKind::Entry,
                path: git_file_last_name(&clean),
            });
        }
    }
    Ok(items)
}

struct GitCommitState {
    message: String,
    files: Vec<String>,
    list_state: ListState,
    content: Vec<String>,
    commits: Vec<String>,
    file_area: Option<Rect>,
    content_area: Option<Rect>,
    content_scroll: u16,
    last_click: Option<(Instant, usize)>,
    input_mode: bool,
    repo_root: PathBuf,
}

impl GitCommitState {
    fn new(_ctx: &mut AppContext) -> anyhow::Result<Self> {
        let repo_root = git::repo_root()?;
        let staged = system_logged(
            "GitCommit",
            &git_cmd_at(&repo_root, "diff --name-only --staged"),
        )?;
        let mut files = Vec::new();
        for line in staged.lines() {
            if !line.trim().is_empty() {
                files.push(format!("s {}", line));
            }
        }
        if files.is_empty() {
            files.push("< Nothing >".to_string());
        }
        let commits =
            git::commit_list_at(&repo_root).unwrap_or_else(|_| vec!["< There is no commit >".to_string()]);
        let mut state = Self {
            message: String::new(),
            files,
            list_state: ListState::default(),
            content: vec!["< Nothing to display >".to_string()],
            commits,
            file_area: None,
            content_area: None,
            content_scroll: 0,
            last_click: None,
            input_mode: true,
            repo_root,
        };
        state.list_state.select(Some(0));
        state.load_content()?;
        Ok(state)
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Length(5),
                Constraint::Length(1),
                Constraint::Min(5),
            ])
            .split(f.size());

        let input_title = if self.input_mode {
            "Commit message (input)"
        } else {
            "Commit message"
        };
        let prompt = "Input commit message => ";
        let input = Paragraph::new(format!("{}{}", prompt, self.message))
            .block(Block::default().title(input_title));
        f.render_widget(input, layout[0]);
        if self.input_mode {
            let inner = layout[0].inner(&Margin {
                horizontal: 0,
                vertical: 1,
            });
            let cursor_x = inner
                .x
                .saturating_add(prompt.len() as u16)
                .saturating_add(self.message.len() as u16)
                .min(inner.x + inner.width.saturating_sub(1));
            let cursor_y = inner.y.min(inner.y + inner.height.saturating_sub(1));
            f.set_cursor(cursor_x, cursor_y);
        }

        let file_items: Vec<ListItem> = self.files.iter().map(|s| ListItem::new(s.as_str())).collect();
        let file_list = List::new(file_items)
            .block(Block::default().title("Files"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(file_list, layout[1], &mut self.list_state);
        self.file_area = Some(layout[1]);

        let commit_items: Vec<ListItem> = self
            .commits
            .iter()
            .map(|s| ListItem::new(s.as_str()))
            .collect();
        let commit_list = List::new(commit_items)
            .block(Block::default().title("Commits"));
        f.render_widget(commit_list, layout[2]);

        let separator = "-".repeat(layout[3].width as usize);
        let sep = Paragraph::new(separator).style(Style::default().fg(Color::DarkGray));
        f.render_widget(sep, layout[3]);

        let diff_lines = format_diff_lines(&self.content, layout[4].width);
        let view = Paragraph::new(Text::from(diff_lines)).block(Block::default().title("Diff"));
        f.render_widget(view.scroll((self.content_scroll, 0)), layout[4]);
        self.content_area = Some(layout[4]);
    }

    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                }
                KeyCode::F(4) => {
                    return Ok(Action::Switch(Screen::GitStatus(GitStatusState::new(ctx)?)));
                }
                KeyCode::Down => {
                    self.next()?;
                }
                KeyCode::Up => {
                    self.prev()?;
                }
                KeyCode::Enter => {
                    if self.message.trim().is_empty() {
                        return Ok(Action::None);
                    }
                let status = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&self.repo_root)
                    .arg("commit")
                    .arg("-m")
                    .arg(&self.message)
                    .status()?;
                if !status.success() {
                    return Err(anyhow::anyhow!("git commit failed"));
                }
                return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
            }
                KeyCode::Backspace => {
                    self.message.pop();
                }
                KeyCode::Char(c) => {
                    if !c.is_control() {
                        self.message.push(c);
                    }
                }
                _ => {}
            }
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(Action::Switch(Screen::GitStatus(GitStatusState::new(ctx)?)));
            }
            KeyCode::F(4) => {
                return Ok(Action::Switch(Screen::GitStatus(GitStatusState::new(ctx)?)));
            }
            KeyCode::Char('i') => {
                self.input_mode = true;
            }
            KeyCode::Char('j') | KeyCode::Down => self.next()?,
            KeyCode::Char('k') | KeyCode::Up => self.prev()?,
            KeyCode::Char('A') | KeyCode::Char('a') => {
                let name = self.focus_file_name().unwrap_or_default();
                if !name.is_empty() {
                    system(&git_cmd_at(&self.repo_root, &format!("add \"{}\"", name)))?;
                    *self = GitCommitState::new(ctx)?;
                }
            }
            KeyCode::Char('R') => {
                let name = self.focus_file_name().unwrap_or_default();
                if !name.is_empty() {
                    system(&git_cmd_at(&self.repo_root, &format!("reset \"{}\"", name)))?;
                    *self = GitCommitState::new(ctx)?;
                }
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn focus_file_name(&self) -> Option<String> {
        let idx = self.list_state.selected()?;
        let line = self.files.get(idx)?.clone();
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            Some(parts[1].trim().to_string())
        } else {
            None
        }
    }

    fn load_content(&mut self) -> anyhow::Result<()> {
        if let Some(name) = self.focus_file_name() {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&self.repo_root)
                .arg("diff")
                .arg("--color")
                .arg("--staged")
                .arg("--")
                .arg(&name)
                .output();
            let (stdout, stderr) = match output {
                Ok(out) => (
                    String::from_utf8_lossy(&out.stdout).to_string(),
                    String::from_utf8_lossy(&out.stderr).to_string(),
                ),
                Err(_) => ("".to_string(), "< no diff >".to_string()),
            };
            let body = if stdout.trim().is_empty() && !stderr.trim().is_empty() {
                stderr
            } else if stdout.trim().is_empty() {
                "< no diff >".to_string()
            } else {
                stdout
            };
            self.content = strip_ansi(body)
                .replace('\t', "    ")
                .lines()
                .map(|s| s.to_string())
                .collect();
        }
        Ok(())
    }

    fn next(&mut self) -> anyhow::Result<()> {
        let i = match self.list_state.selected() {
            Some(i) => min(i + 1, self.files.len().saturating_sub(1)),
            None => 0,
        };
        self.list_state.select(Some(i));
        self.load_content()
    }

    fn prev(&mut self) -> anyhow::Result<()> {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
        self.load_content()
    }

    fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.file_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.files.len() {
                                self.list_state.select(Some(idx));
                                let _ = self.load_content();
                                if matches!(me.kind, MouseEventKind::Down(_)) {
                                    let _ = is_double_click(&mut self.last_click, idx);
                                }
                            }
                        }
                        if matches!(me.kind, MouseEventKind::ScrollDown) {
                            let _ = self.next();
                        } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                            let _ = self.prev();
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(area) = self.content_area {
            if area.contains(mouse_pos(&me)) {
                if matches!(me.kind, MouseEventKind::ScrollDown) {
                    self.content_scroll = self.content_scroll.saturating_add(3);
                } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                    self.content_scroll = self.content_scroll.saturating_sub(3);
                }
            }
        }
        Ok(Action::None)
    }
}

struct RegListState {
    items: Vec<RegItem>,
    list_state: ListState,
    filter: String,
    list_area: Option<Rect>,
    log_area: Option<Rect>,
    pull_rx: Option<mpsc::Receiver<PullEvent>>,
    pull_infos: HashMap<String, RepoPullInfo>,
    pull_total: usize,
    pull_done: usize,
    status_rx: Option<mpsc::Receiver<StatusEvent>>,
    status_infos: HashMap<String, RepoStatusInfo>,
    show_log: bool,
    log_path: Option<String>,
    log_scroll: u16,
    confirm_delete: bool,
    confirm_target: Option<String>,
    last_click: Option<(Instant, usize)>,
}

impl RegListState {
    fn new(ctx: &mut AppContext) -> anyhow::Result<Self> {
        let mut items = ctx.config.path.clone();
        items.sort_by_key(|i| i.path.clone());
        let mut state = Self {
            items,
            list_state: ListState::default(),
            filter: String::new(),
            list_area: None,
            log_area: None,
            pull_rx: None,
            pull_infos: HashMap::new(),
            pull_total: 0,
            pull_done: 0,
            status_rx: None,
            status_infos: HashMap::new(),
            show_log: false,
            log_path: None,
            log_scroll: 0,
            confirm_delete: false,
            confirm_target: None,
            last_click: None,
        };
        state.list_state.select(Some(0));
        state.start_status_check();
        Ok(state)
    }

    fn start_status_check(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.status_rx = Some(rx);
        self.status_infos.clear();

        let targets: Vec<String> = self.items.iter()
            .filter(|i| i.repo)
            .map(|i| i.path.clone())
            .collect();

        let sem = Arc::new(Semaphore::new(10));
        for path in targets {
            let tx = tx.clone();
            let sem = sem.clone();
            thread::spawn(move || {
                sem.acquire();
                run_git_status_check(path, tx);
                sem.release();
            });
        }
    }

    fn drain_status_events(&mut self) {
        let Some(rx) = &self.status_rx else { return };
        loop {
            match rx.try_recv() {
                Ok(ev) => {
                    if let Some(info) = ev.info {
                        self.status_infos.insert(ev.path, info);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status_rx = None;
                    break;
                }
            }
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        self.drain_pull_events();
        self.drain_status_events();
        let header = if self.pull_total > 0 {
            format!("Repo list - pull {}/{}", self.pull_done, self.pull_total)
        } else {
            "Repo list".to_string()
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(0),    // Body (List)
                Constraint::Length(2), // Filter (Bottom)
            ])
            .split(f.size());

        let title_widget = Paragraph::new(Line::from(Span::styled(
            format!(" >> {}", header),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        f.render_widget(title_widget, layout[0]);

        if self.show_log {
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(layout[1]);
            self.render_list("".to_string(), body[0], f);
            self.render_log(body[1], f);
        } else {
            self.render_list("".to_string(), layout[1], f);
        }

        let input_title = if self.confirm_delete {
            "Delete? (y/n)"
        } else {
            "Filter"
        };
        let input_text = if self.confirm_delete {
            self.confirm_target.clone().unwrap_or_default()
        } else {
            format!("{}{}", INPUT_PREFIX, self.filter)
        };
        let input = Paragraph::new(input_text).block(Block::default().title(input_title));
        f.render_widget(input, layout[2]);
    }

    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        if self.confirm_delete {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(path) = self.confirm_target.clone() {
                        ctx.reg_remove(&path)?;
                        self.items.retain(|i| i.path != path);
                    }
                    self.confirm_delete = false;
                    self.confirm_target = None;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.confirm_delete = false;
                    self.confirm_target = None;
                }
                _ => {}
            }
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    self.list_state.select(Some(0));
                    return Ok(Action::None);
                }
                if self.show_log {
                    self.show_log = false;
                    return Ok(Action::None);
                }
                return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
            }
            KeyCode::Char('Q') => {
                return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
            }
            KeyCode::Down => self.next(),
            KeyCode::Up => self.prev(),
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::ALT) => self.next(),
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::ALT) => self.prev(),
            KeyCode::Char('P') => {
                if !self.pull_active() {
                    self.start_pull(ctx.config.is_pull_rebase);
                }
            }
            KeyCode::Char('T') => {
                if let Some(item) = self.focus_item() {
                    let target = item.path.clone();
                    with_terminal_pause(|| {
                        let old = std::env::current_dir()?;
                        if std::env::set_current_dir(&target).is_ok() {
                            let _ = system_stream("tig");
                            let _ = std::env::set_current_dir(old);
                        }
                        Ok(())
                    })?;
                }
            }
            KeyCode::Char('D') => {
                if let Some(item) = self.focus_item() {
                    self.confirm_delete = true;
                    self.confirm_target = Some(item.path);
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.focus_item() {
                    self.show_log = true;
                    self.log_path = Some(item.path);
                    self.log_scroll = 0;
                }
            }
            KeyCode::PageDown => {
                if self.show_log {
                    self.log_scroll = self.log_scroll.saturating_add(5);
                }
            }
            KeyCode::PageUp => {
                if self.show_log {
                    self.log_scroll = self.log_scroll.saturating_sub(5);
                }
            }
            KeyCode::Delete => {
                if let Some(item) = self.focus_item() {
                    ctx.reg_remove(&item.path)?;
                    self.items.retain(|i| i.path != item.path);
                }
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.list_state.select(Some(0));
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.list_state.select(Some(0));
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.filtered_items().len() {
                                self.list_state.select(Some(idx));
                                if self.show_log {
                                    self.log_path = self.focus_item().map(|i| i.path);
                                    self.log_scroll = 0;
                                }
                                if matches!(me.kind, MouseEventKind::Down(_)) {
                                    if is_double_click(&mut self.last_click, idx) {
                                        if let Some(item) = self.focus_item() {
                                            self.show_log = true;
                                            self.log_path = Some(item.path);
                                            self.log_scroll = 0;
                                        }
                                    }
                                }
                            }
                        }
                        if matches!(me.kind, MouseEventKind::ScrollDown) {
                            self.next();
                        } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                            self.prev();
                        } else if matches!(me.kind, MouseEventKind::Down(_)) {
                            if self.show_log {
                                self.log_path = self.focus_item().map(|i| i.path);
                                self.log_scroll = 0;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(area) = self.log_area {
            if area.contains(mouse_pos(&me)) {
                if matches!(me.kind, MouseEventKind::ScrollDown) {
                    self.log_scroll = self.log_scroll.saturating_add(3);
                } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                    self.log_scroll = self.log_scroll.saturating_sub(3);
                }
            }
        }
        Ok(Action::None)
    }

    fn filtered_items(&self) -> Vec<RegItem> {
        if self.filter.trim().is_empty() {
            return self.items.clone();
        }
        let filter = self.filter.to_lowercase();
        self.items
            .iter()
            .filter(|i| {
                i.names
                    .iter()
                    .any(|n| n.to_lowercase().contains(&filter))
            })
            .cloned()
            .collect()
    }

    fn focus_item(&self) -> Option<RegItem> {
        let idx = self.list_state.selected()?;
        self.filtered_items().get(idx).cloned()
    }

    fn next(&mut self) {
        let len = self.filtered_items().len();
        let i = match self.list_state.selected() {
            Some(i) => min(i + 1, len.saturating_sub(1)),
            None => 0,
        };
        self.list_state.select(Some(i));
        if self.show_log {
            self.log_path = self.focus_item().map(|i| i.path);
            self.log_scroll = 0;
        }
    }

    fn prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
        if self.show_log {
            self.log_path = self.focus_item().map(|i| i.path);
            self.log_scroll = 0;
        }
    }

    fn render_list(&mut self, header: String, area: Rect, f: &mut ratatui::Frame) {
        let filtered = self.filtered_items();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|i| {
                let (status, msg, style) = if let Some(info) = self.pull_infos.get(&i.path) {
                    let status = info.status.label().to_string();
                    let msg = match &info.status {
                        PullStatus::Done { code: _, message } => message.clone(),
                        _ => None,
                    };
                    let style = match &info.status {
                        PullStatus::Pending => Style::default().fg(Color::DarkGray),
                        PullStatus::Running => Style::default().fg(Color::Yellow),
                        PullStatus::Done { code, .. } => {
                            if *code == 0 {
                                Style::default().fg(Color::Green)
                            } else {
                                Style::default().fg(Color::Red)
                            }
                        }
                    };
                    (status, msg, style)
                } else {
                    ("".to_string(), None, Style::default())
                };
                
                let mut label = i.path.clone();
                if let Some(info) = self.status_infos.get(&i.path) {
                    if info.dirty {
                        label.push_str(" *");
                    }
                    if !info.branch.is_empty() {
                        label.push_str(&format!(" [{}", info.branch));
                        if !info.upstream.is_empty() {
                            let clean_upstream = info.upstream.replace("refs/remotes/", "");
                            label.push_str(&format!(" -> {}", clean_upstream));
                        }
                        label.push(']');
                    }
                }

                let text = if status.is_empty() {
                    label
                } else if status == "ERR" {
                    if let Some(msg) = msg {
                        format!("{} [{}] {}", label, status, msg)
                    } else {
                        format!("{} [{}]", label, status)
                    }
                } else {
                    format!("{} [{}]", label, status)
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let mut list = List::new(items)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(64, 64, 64))
                    .fg(Color::Cyan),
            )
            .highlight_symbol(">> ");

        if !header.is_empty() {
            list = list.block(
                Block::default()
                    .title(header)
                    .title_style(Style::default().fg(Color::Green)),
            );
        }

        f.render_stateful_widget(list, area, &mut self.list_state);
        self.list_area = Some(area);
    }

    fn render_log(&mut self, area: Rect, f: &mut ratatui::Frame) {
        let title = self
            .log_path
            .clone()
            .unwrap_or_else(|| "< no selection >".to_string());
        let lines = if let Some(path) = &self.log_path {
            self.pull_infos
                .get(path)
                .map(|info| info.log.clone())
                .unwrap_or_else(|| vec!["< No log >".to_string()])
        } else {
            vec!["< No log >".to_string()]
        };
        let text = Text::from(lines.join("\n"));
        let view = Paragraph::new(text)
            .block(Block::default().title(title))
            .scroll((self.log_scroll, 0));
        f.render_widget(view, area);
        self.log_area = Some(area);
    }

    fn start_pull(&mut self, is_rebase: bool) {
        let (tx, rx) = mpsc::channel();
        let targets: Vec<RegItem> = self.items.iter().filter(|i| i.repo).cloned().collect();
        self.pull_total = targets.len();
        self.pull_done = 0;
        self.pull_rx = Some(rx);
        self.pull_infos.clear();
        for item in &targets {
            self.pull_infos.insert(
                item.path.clone(),
                RepoPullInfo {
                    status: PullStatus::Pending,
                    log: vec![],
                },
            );
        }

        let sem = Arc::new(Semaphore::new(5));
        for item in targets {
            let tx = tx.clone();
            let sem = sem.clone();
            let path = item.path.clone();
            thread::spawn(move || {
                sem.acquire();
                let _ = tx.send(PullEvent::started(path.clone()));
                let (code, message) = run_git_pull(&path, is_rebase, &tx);
                let _ = tx.send(PullEvent::finished(path.clone(), code, message));
                sem.release();
            });
        }
    }

    fn pull_active(&self) -> bool {
        self.pull_total > 0 && self.pull_done < self.pull_total
    }

    fn drain_pull_events(&mut self) {
        let Some(rx) = &self.pull_rx else { return };
        loop {
            match rx.try_recv() {
                Ok(ev) => match ev.kind {
                    PullEventKind::Started => {
                        if let Some(info) = self.pull_infos.get_mut(&ev.path) {
                            info.status = PullStatus::Running;
                        }
                    }
                    PullEventKind::Line(line) => {
                        if let Some(info) = self.pull_infos.get_mut(&ev.path) {
                            info.log.push(line);
                            if info.log.len() > 2000 {
                                let excess = info.log.len() - 2000;
                                info.log.drain(0..excess);
                            }
                        }
                    }
                    PullEventKind::Finished(code, message) => {
                        if let Some(info) = self.pull_infos.get_mut(&ev.path) {
                            info.status = PullStatus::Done { code, message };
                        }
                        self.pull_done = self.pull_done.saturating_add(1);
                    }
                },
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pull_rx = None;
                    break;
                }
            }
        }
    }
}

struct RepoPullInfo {
    status: PullStatus,
    log: Vec<String>,
}

enum PullStatus {
    Pending,
    Running,
    Done { code: i32, message: Option<String> },
}

impl PullStatus {
    fn label(&self) -> &'static str {
        match self {
            PullStatus::Pending => "PEND",
            PullStatus::Running => "RUN",
            PullStatus::Done { code, .. } => {
                if *code == 0 {
                    "OK"
                } else {
                    "ERR"
                }
            }
        }
    }
}

struct PullEvent {
    path: String,
    kind: PullEventKind,
}

enum PullEventKind {
    Started,
    Line(String),
    Finished(i32, Option<String>),
}

impl PullEvent {
    fn started(path: String) -> Self {
        Self {
            path,
            kind: PullEventKind::Started,
        }
    }

    fn finished(path: String, code: i32, message: Option<String>) -> Self {
        Self {
            path,
            kind: PullEventKind::Finished(code, message),
        }
    }
}

struct Semaphore {
    max: usize,
    lock: Mutex<usize>,
    cvar: Condvar,
}

impl Semaphore {
    fn new(max: usize) -> Self {
        Self {
            max,
            lock: Mutex::new(0),
            cvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.lock.lock().expect("lock");
        while *count >= self.max {
            count = self.cvar.wait(count).expect("wait");
        }
        *count += 1;
    }

    fn release(&self) {
        let mut count = self.lock.lock().expect("lock");
        if *count > 0 {
            *count -= 1;
        }
        self.cvar.notify_one();
    }
}

fn run_git_pull(path: &str, is_rebase: bool, tx: &mpsc::Sender<PullEvent>) -> (i32, Option<String>) {
    let cmd = if is_rebase {
        "git pull -r 2>&1"
    } else {
        "git pull 2>&1"
    };
    let mut child = match std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(path)
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            let _ = tx.send(PullEvent {
                path: path.to_string(),
                kind: PullEventKind::Line(format!("spawn error: {}", err)),
            });
            return (1, Some(err.to_string()));
        }
    };
    let mut last_line: Option<String> = None;
    let mut err_line: Option<String> = None;
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            let line_trim = line.trim().to_string();
            if !line_trim.is_empty() {
                last_line = Some(line_trim.clone());
                if line_trim.to_lowercase().starts_with("error") {
                    err_line = Some(line_trim.clone());
                }
            }
            let _ = tx.send(PullEvent {
                path: path.to_string(),
                kind: PullEventKind::Line(line),
            });
        }
    }
    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    let message = if code == 0 { None } else { err_line.or(last_line) };
    (code, message)
}

fn format_diff_lines(lines: &[String], width: u16) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut first_hunk = true;
    let rule_len = width.max(1) as usize;
    for line in lines {
        if line.starts_with("@@") {
            if !first_hunk {
                out.push(Line::from(Span::styled(
                    "-".repeat(rule_len),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            first_hunk = false;
            out.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::Cyan),
            )));
            continue;
        }
        let style = if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
        {
            Style::default().fg(Color::Yellow)
        } else if line.starts_with('+') {
            Style::default().fg(Color::Green)
        } else if line.starts_with('-') {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };
        out.push(Line::from(Span::styled(line.clone(), style)));
    }
    out
}

struct GotoState {
    items: Vec<RegItem>,
    local_dirs: Vec<RegItem>,
    list_state: ListState,
    filter: String,
    list_area: Option<Rect>,
    last_click: Option<(Instant, usize)>,
}

impl GotoState {
    fn new(ctx: &mut AppContext) -> anyhow::Result<Self> {
        let mut items = ctx.config.path.clone();
        items.sort_by_key(|i| i.path.clone());
        let cwd = std::env::current_dir()?;
        let local_dirs = walk_dirs(&cwd, &[".git", "node_modules", ".pnpm-store"], 100)
            .into_iter()
            .map(|p| RegItem {
                names: vec![p.to_string_lossy().to_string()],
                path: p.to_string_lossy().to_string(),
                groups: vec![],
                repo: false,
            })
            .collect::<Vec<_>>();
        let mut state = Self {
            items,
            local_dirs,
            list_state: ListState::default(),
            filter: String::new(),
            list_area: None,
            last_click: None,
        };
        state.list_state.select(Some(0));
        Ok(state)
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(0),    // List
                Constraint::Length(2), // Filter
            ])
            .split(f.size());

        let title_widget = Paragraph::new(Line::from(Span::styled(
            " >> Goto",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        f.render_widget(title_widget, layout[0]);

        let filtered = self.filtered_items();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|i| ListItem::new(i.path.as_str()))
            .collect();
        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(64, 64, 64))
                    .fg(Color::Cyan),
            )
            .highlight_symbol(">> ");
        f.render_stateful_widget(list, layout[1], &mut self.list_state);
        self.list_area = Some(layout[1]);

        let input = Paragraph::new(format!("{}{}", INPUT_PREFIX, self.filter))
            .block(Block::default().title("Filter"));
        f.render_widget(input, layout[2]);
    }

    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
            }
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.prev(),
            KeyCode::Enter => {
                if let Some(item) = self.focus_item() {
                    let path = if Path::new(&item.path).is_absolute() {
                        PathBuf::from(&item.path)
                    } else {
                        std::env::current_dir()?.join(&item.path)
                    };
                    std::env::set_current_dir(path)?;
                    return Ok(Action::Switch(Screen::Main(MainState::new(ctx)?)));
                }
            }
            KeyCode::Char('U') => {
                if let Some(parent) = std::env::current_dir()?.parent() {
                    std::env::set_current_dir(parent)?;
                }
                *self = GotoState::new(ctx)?;
            }
            KeyCode::Char(c) => {
                if c.is_ascii_graphic() || c == ' ' {
                    self.filter.push(c);
                }
            }
            KeyCode::Backspace => {
                self.filter.pop();
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.filtered_items().len() {
                                self.list_state.select(Some(idx));
                                if matches!(me.kind, MouseEventKind::Down(_)) {
                                    if is_double_click(&mut self.last_click, idx) {
                                        if let Some(item) = self.focus_item() {
                                            let path = if Path::new(&item.path).is_absolute() {
                                                PathBuf::from(&item.path)
                                            } else {
                                                std::env::current_dir()?.join(&item.path)
                                            };
                                            std::env::set_current_dir(path)?;
                                            return Ok(Action::Switch(Screen::Main(MainState::new(
                                                _ctx,
                                            )?)));
                                        }
                                    }
                                }
                            }
                        }
                        if matches!(me.kind, MouseEventKind::ScrollDown) {
                            self.next();
                        } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                            self.prev();
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(Action::None)
    }

    fn filtered_items(&self) -> Vec<RegItem> {
        let mut combined = self.items.clone();
        combined.extend(self.local_dirs.clone());
        if self.filter.trim().is_empty() {
            return combined;
        }
        let filter = self.filter.to_lowercase();
        let list: Vec<String> = filter.split_whitespace().map(|s| s.to_string()).collect();
        let mut output = combined;
        output.sort_by_key(|i| {
            let base = Path::new(&i.path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            std::cmp::Reverse(match_disorder_count(&base.to_lowercase(), &list))
        });
        output
            .into_iter()
            .filter(|i| match_disorder(&i.path.to_lowercase(), &list))
            .collect()
    }

    fn focus_item(&self) -> Option<RegItem> {
        let idx = self.list_state.selected()?;
        self.filtered_items().get(idx).cloned()
    }

    fn next(&mut self) {
        let len = self.filtered_items().len();
        let i = match self.list_state.selected() {
            Some(i) => min(i + 1, len.saturating_sub(1)),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}


fn strip_ansi(input: String) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            while let Some(c) = chars.next() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn git_file_last_name(line: &str) -> Option<String> {
    let text = line.trim();
    let first_space = text.find(' ')?;
    let rest = text[first_space + 1..].trim();
    if let Some(pos) = rest.rfind(" -> ") {
        let target = &rest[pos + 4..];
        return Some(unquote_path(target));
    }
    Some(unquote_path(rest))
}

fn git_cmd_at(root: &Path, cmd: &str) -> String {
    format!("git -C \"{}\" {}", root.to_string_lossy(), cmd)
}

fn unquote_path(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].replace("\\\"", "\"")
    } else {
        trimmed.to_string()
    }
}

fn file_size(path: &str) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn mouse_pos(me: &MouseEvent) -> Position {
    Position::new(me.column, me.row)
}

fn is_double_click(last: &mut Option<(Instant, usize)>, idx: usize) -> bool {
    let now = Instant::now();
    if let Some((t, prev)) = *last {
        if prev == idx && now.duration_since(t) <= Duration::from_millis(400) {
            *last = None;
            return true;
        }
    }
    *last = Some((now, idx));
    false
}

fn with_terminal_pause<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    disable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    let result = f();
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        EnableMouseCapture
    )?;
    enable_raw_mode()?;
    REDRAW_REQUEST.store(true, Ordering::SeqCst);
    result
}


#[derive(Clone, Copy)]
enum GitAction {
    Fetch,
    Merge,
    Status,
    Update,
}

struct GitActor<'a> {
    ctx: &'a mut AppContext,
    repo_list: Vec<RegItem>,
}

impl<'a> GitActor<'a> {
    fn new(ctx: &'a mut AppContext) -> Self {
        let repo_list = ctx.config.path.iter().filter(|r| r.repo).cloned().collect();
        Self { ctx, repo_list }
    }

    fn action(&mut self, action: GitAction, target: Option<&str>) -> anyhow::Result<bool> {
        if let Some(target) = target {
            return self.apply(action, target);
        }
        for repo in self.repo_list.clone() {
            if let Some(name) = repo.names.get(0) {
                if !self.apply(action, name)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn apply(&mut self, action: GitAction, target: &str) -> anyhow::Result<bool> {
        match action {
            GitAction::Fetch => self.act_fetch(target),
            GitAction::Merge => self.act_merge(target),
            GitAction::Status => self.act_status(target),
            GitAction::Update => self.act_update(target),
        }
    }

    fn act_fetch(&mut self, name: &str) -> anyhow::Result<bool> {
        let path = self.change_path(name)?;
        println!("fetch --prune - {}", path.to_string_lossy());
        let _ = system("git fetch --prune");
        Ok(true)
    }

    fn act_merge(&mut self, name: &str) -> anyhow::Result<bool> {
        let path = self.change_path(name)?;
        let branch = git::get_current_branch()?;
        let remote = match git::get_tracking_branch() {
            Some(r) => r,
            None => {
                println!("{} DONT'T HAVE TRACKING branch", branch);
                return Ok(false);
            }
        };
        let same = self.check_same_with(name, &branch, &remote)?;
        if same {
            return Ok(true);
        }
        let diff = git::check_rebaseable(&branch, &remote)?;
        if !diff.is_empty() {
            println!("NOT be able to fast forward - {}", path.to_string_lossy());
        } else {
            println!("merge with {} - {}", remote, path.to_string_lossy());
            let _ = system(&format!("git rebase {}", remote));
        }
        Ok(true)
    }

    fn act_status(&mut self, name: &str) -> anyhow::Result<bool> {
        let _path = self.change_path(name)?;
        if !self.stash_check(name)? {
            return Ok(false);
        }
        let branch = git::get_current_branch()?;
        let remote = match git::get_tracking_branch() {
            Some(r) => r,
            None => {
                println!("{} DONT'T HAVE TRACKING branch", branch);
                return Ok(false);
            }
        };
        let same = self.check_same_with(name, &branch, &remote)?;
        if same {
            if let Ok(out) = system("git status -s") {
                if !out.is_empty() {
                    println!("{out}");
                }
            }
        } else {
            let diff = git::check_rebaseable(&branch, &remote)?;
            if diff.is_empty() {
                println!("Be able to fast-forward...");
            } else {
                println!("NOT be able to fast forward");
            }
        }
        Ok(true)
    }

    fn act_update(&mut self, name: &str) -> anyhow::Result<bool> {
        let _ = self.act_pull(name)?;
        let _ = self.act_status(name)?;
        Ok(true)
    }

    fn act_pull(&mut self, name: &str) -> anyhow::Result<bool> {
        let path = self.change_path(name)?;
        let mut cmd = "pull".to_string();
        if self.ctx.config.is_pull_rebase {
            cmd.push_str(" -r");
        }
        println!("{} - {}", cmd, path.to_string_lossy());
        let (_out, code) = system_safe(&format!("git {}", cmd));
        Ok(code == 0)
    }

    fn stash_check(&mut self, _name: &str) -> anyhow::Result<bool> {
        let stash = git::stash_get_name_safe("###groupRepo###")?;
        if stash.is_some() {
            println!("YOU HAVE STASH ITEM. PROCESS IT FIRST");
            return Ok(false);
        }
        Ok(true)
    }

    fn check_same_with(
        &mut self,
        name: &str,
        branch: &str,
        remote: &str,
    ) -> anyhow::Result<bool> {
        let rev = git::rev(branch)?;
        let rev2 = git::rev(&format!("remotes/{}", remote))?;
        if rev == rev2 {
            println!("{} -> {} is same to {}", name, branch, remote);
            return Ok(true);
        }
        let common = git::common_parent_rev(branch, remote)?;
        if common != rev2 {
            println!("{} -> Different", name);
            return Ok(false);
        }
        let gap = git::commit_gap(branch, remote)?;
        println!(
            "Your local branch({}) is forward than {}[{} commits]",
            branch, remote, gap
        );
        println!("{}", git::commit_log_between(branch, remote)?);
        Ok(true)
    }

    fn change_path(&mut self, name: &str) -> anyhow::Result<PathBuf> {
        let path = if name.starts_with('/') {
            PathBuf::from(name)
        } else {
            let repo = self
                .repo_list
                .iter()
                .find(|r| r.names.contains(&name.to_string()))
                .ok_or_else(|| anyhow::anyhow!("Can't find repo[name:{}]", name))?;
            PathBuf::from(&repo.path)
        };
        if !path.is_dir() {
            return Err(anyhow::anyhow!("{} doesn't exist", path.to_string_lossy()));
        }
        std::env::set_current_dir(&path)?;
        Ok(path)
    }
}

struct RepoStatusInfo {
    branch: String,
    upstream: String,
    dirty: bool,
}

struct StatusEvent {
    path: String,
    info: Option<RepoStatusInfo>,
}

fn run_git_status_check(path: String, tx: mpsc::Sender<StatusEvent>) {
    let output_branch = std::process::Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&path)
        .output();
    
    let branch = match output_branch {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => return, // Not a repo or git missing
    };

    let output_upstream = std::process::Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(&path)
        .output();
    
    let upstream = match output_upstream {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => String::new(),
    };

    let output_status = std::process::Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(&path)
        .output();
    
    let dirty = match output_status {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => false,
    };

    let _ = tx.send(StatusEvent {
        path,
        info: Some(RepoStatusInfo {
            branch,
            upstream,
            dirty,
        }),
    });
}
