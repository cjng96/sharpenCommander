use std::cmp::min;
use std::path::{PathBuf};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::app::{open_in_editor, AppContext};
use crate::git;
use crate::system::{app_log};
use crate::ui::common::{Action, Screen, INPUT_PREFIX, mouse_pos, is_double_click, with_terminal_pause};

pub struct MainState {
    pub cwd: PathBuf,
    pub items: Vec<DirEntry>,
    pub list_state: ListState,
    pub input: String,
    pub input_mode: bool,
    pub cmd_list: Vec<String>,
    pub work_list: Vec<PathBuf>,
    pub work_idx: usize,
    pub list_area: Option<Rect>,
    pub last_click: Option<(Instant, usize)>,
    pub registered_paths: Vec<String>,
    pub confirm_delete: bool,
    pub confirm_target: Option<String>,
}

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

impl MainState {
    pub fn new(ctx: &mut AppContext) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let registered_paths = ctx.config.path.iter().map(|i| i.path.clone()).collect();
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
            registered_paths,
            confirm_delete: false,
            confirm_target: None,
        };
        state.refresh();
        Ok(state)
    }

    pub fn refresh(&mut self) {
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

        self.list_state.select(Some(0));
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
                }
            }
            return;
        }
        let target = self.cwd.join(name);
        if target.is_dir() {
            if std::env::set_current_dir(&target).is_ok() {
                self.cwd = target;
                self.refresh();
            }
        }
    }

    fn focus_editor_target(&self) -> Option<PathBuf> {
        let idx = self.list_state.selected()?;
        let entry = self.items.get(idx)?;
        if entry.name == ".." {
            return Some(self.cwd.clone());
        }
        let path = self.cwd.join(&entry.name);
        if path.is_file() || path.is_dir() {
            Some(path)
        } else {
            None
        }
    }

    pub fn render(&mut self, f: &mut ratatui::Frame) {
        let cwd_str = self.cwd.to_string_lossy();
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
            .split(f.size());

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(85), // List
                Constraint::Percentage(15), // Cmd
            ])
            .split(layout[0]);

        let header = format!(" >> sc V{} - {}", env!("CARGO_PKG_VERSION"), cwd_str);
        let list_block = Block::default().title(header);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| {
                let mut style = Style::default();
                if i.is_dir {
                    let full_path = self.cwd.join(&i.name).to_string_lossy().to_string();
                    if self.registered_paths.contains(&full_path) {
                        style = style.fg(Color::White);
                    } else {
                        style = style.fg(Color::Green);
                    }
                    ListItem::new(i.name.as_str()).style(style)
                } else if i.name.ends_with(".lua") {
                    let text = format!("{:<20} [Play]", i.name);
                    ListItem::new(text).style(style.fg(Color::Yellow))
                } else {
                    ListItem::new(i.name.as_str()).style(style)
                }
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

        if self.confirm_delete {
            let area = crate::ui::common::centered_rect(40, 7, f.size());
            f.render_widget(Clear, area);
            let target = self.confirm_target.clone().unwrap_or_default();
            let text = vec![
                Line::from(vec![
                    Span::raw("Delete from repo list? "),
                    Span::styled(target, Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
                ]),
                Line::from(Span::styled("(y) Yes / (N) No", Style::default().fg(Color::DarkGray))),
            ];
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Confirmation ")
                .border_style(Style::default().fg(Color::DarkGray));
            let p = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(p, area);
        }
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        if self.confirm_delete {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(path) = self.confirm_target.clone() {
                        ctx.reg_remove(&path)?;
                        self.registered_paths.retain(|p| p != &path);
                    }
                    self.confirm_delete = false;
                    self.confirm_target = None;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Enter => {
                    self.confirm_delete = false;
                    self.confirm_target = None;
                }
                _ => {} // Ignore other keys when confirming deletion
            }
            return Ok(Action::None);
        }

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
                _ => {} // Ignore other keys in input mode
            }
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Char('q') => return Ok(Action::Exit),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_prev(),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('g') | KeyCode::Char('G') => {
                return Ok(Action::Switch(Screen::Goto(Box::new(crate::ui::goto_ui::GotoState::new(ctx)?))));
            }
            KeyCode::Char('H') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.enter_dir("..");
            }
            KeyCode::Char('L') if key.modifiers.contains(KeyModifiers::ALT) => {
                if let Some(name) = self.focus_name() {
                    self.enter_dir(&name);
                }
            }
            KeyCode::Char('U') | KeyCode::Char('.') | KeyCode::Left | KeyCode::Char('h') => {
                self.enter_dir("..");
            }
            KeyCode::Enter => {
                if let Some(name) = self.focus_name() {
                    self.enter_dir(&name);
                }
            }
            KeyCode::Char('E') => {
                if let Some(path) = self.focus_editor_target() {
                    open_in_editor(&ctx.config.edit_app, path.to_string_lossy().as_ref());
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = true;
            }
            KeyCode::Char('L') => {
                return Ok(Action::Switch(Screen::RegList(Box::new(crate::ui::reg_list_ui::RegListState::new(ctx)?))));
            }
            KeyCode::Char('C') => {
                app_log(&format!(
                    "Key C (Main) cwd={}",
                    self.cwd.to_string_lossy()
                ));
                match crate::ui::git_stage_ui::GitStageState::new(ctx) {
                    Ok(state) => return Ok(Action::Switch(Screen::GitStage(Box::new(state)))),
                    Err(err) => {
                        app_log(&format!("Key C (Main) error: {}", err));
                        return Ok(Action::Toast(err.to_string()));
                    }
                }
            }
            KeyCode::Char('R') => {
                if let Some(name) = self.focus_name() {
                    let path = self.cwd.join(&name);
                    if path.is_dir() {
                        let path_str = path.to_string_lossy().to_string();
                        if ctx.reg_find_by_path(&path_str).is_none() {
                            ctx.reg_add(&path_str)?;
                            self.registered_paths.push(path_str.clone());
                            return Ok(Action::Toast(format!("Registered: {}", path_str)));
                        } else {
                            return Ok(Action::Toast(format!("Already registered: {}", path_str)));
                        }
                    }
                }
            }
            KeyCode::Char('D') => {
                if let Some(name) = self.focus_name() {
                    let path = self.cwd.join(&name);
                    let path_str = path.to_string_lossy().to_string();
                    if self.registered_paths.contains(&path_str) {
                        self.confirm_delete = true;
                        self.confirm_target = Some(path_str);
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
                                let _ = crate::system::system_stream("git pull -r");
                                let _ = std::env::set_current_dir(old);
                            }
                            Ok(())
                        })?;
                    }
                }
            }
            KeyCode::Char('T') => {
                match crate::ui::git_history_ui::GitHistoryState::new(ctx) {
                    Ok(state) => return Ok(Action::Switch(Screen::GitHistory(Box::new(state)))),
                    Err(err) => return Ok(Action::Toast(err.to_string())),
                }
            }
            KeyCode::Char('P') => {
                if let Some(name) = self.focus_name() {
                    if name.ends_with(".lua") {
                        let path = self.cwd.join(&name);
                        with_terminal_pause(|| {
                            println!("$ lua {}", name);
                            let _ = crate::system::system_stream(&format!("lua \"{}\"", path.to_string_lossy()));
                            Ok(())
                        })?;
                        return Ok(Action::None);
                    }
                }
                crate::ui::git_push(ctx)?;
            }
            _ => {} // Ignore other keys
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
                return Ok(Action::Switch(Screen::Find(Box::new(crate::ui::find_ui::FindState::from_args(
                    ctx, &args,
                )?))));
            }
            "grep" | "gg" => {
                let args: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
                return Ok(Action::Switch(Screen::Grep(Box::new(crate::ui::grep_ui::GrepState::from_args(
                    ctx, &args,
                )?))));
            }
            "reg" => {
                let path = self.cwd.to_string_lossy().to_string();
                if ctx.reg_find_by_path(&path).is_none() {
                    ctx.reg_add(&path)?;
                    self.registered_paths.push(path.clone());
                    return Ok(Action::Toast(format!("Registered: {}", path)));
                } else {
                    return Ok(Action::Toast(format!("Already registered: {}", path)));
                }
            }
            "st" => {
                let target = parts.get(1).map(|s| *s);
                let mut actor = git::GitActor::new(ctx.config.is_pull_rebase, ctx.config.path.clone());
                actor.action(git::GitAction::Status, target)?;
            }
            "update" => {
                let target = parts.get(1).map(|s| *s);
                let mut actor = git::GitActor::new(ctx.config.is_pull_rebase, ctx.config.path.clone());
                actor.action(git::GitAction::Update, target)?;
            }
            "fetch" => {
                let target = parts.get(1).map(|s| *s);
                let mut actor = git::GitActor::new(ctx.config.is_pull_rebase, ctx.config.path.clone());
                actor.action(git::GitAction::Fetch, target)?;
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

    pub fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.items.len() {
                                self.list_state.select(Some(idx));
                                let is_play_click = me.column >= inner.x + 20;
                                let mut run_lua = None;
                                let mut enter_target = None;

                                if let Some(entry) = self.items.get(idx) {
                                    if is_play_click && entry.name.ends_with(".lua") {
                                        run_lua = Some(entry.name.clone());
                                    } else if is_double_click(&mut self.last_click, idx) {
                                        enter_target = Some(entry.name.clone());
                                    }
                                }

                                if let Some(name) = run_lua {
                                    let path = self.cwd.join(&name);
                                    with_terminal_pause(|| {
                                        println!("$ lua {}", name);
                                        let _ = crate::system::system_stream(&format!("lua \"{}\"", path.to_string_lossy()));
                                        Ok(())
                                    })?;
                                } else if let Some(name) = enter_target {
                                    self.enter_dir(&name);
                                }
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        self.select_next();
                    }
                    MouseEventKind::ScrollUp => {
                        self.select_prev();
                    }
                    _ => {} // Ignore other mouse events
                }
            }
        }
        Ok(Action::None)
    }
}

impl crate::ui::common::ScreenState for MainState {
    fn render(&mut self, f: &mut ratatui::Frame) {
        self.render(f);
    }
    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        self.on_key(ctx, key)
    }
    fn on_mouse(&mut self, ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        self.on_mouse(ctx, me)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crossterm::event::KeyEvent;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sc_main_ui_{prefix}_{nanos}_{id}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn make_ctx(edit_app: String) -> crate::app::AppContext {
        let mut config = Config::default();
        config.edit_app = edit_app;
        crate::app::AppContext {
            config,
            config_path: PathBuf::from("unused"),
        }
    }

    fn make_state(cwd: &Path, focused_entry: &str, is_dir: bool) -> MainState {
        let mut list_state = ListState::default();
        list_state.select(Some(1));
        MainState {
            cwd: cwd.to_path_buf(),
            items: vec![
                DirEntry {
                    name: "..".to_string(),
                    is_dir: true,
                },
                DirEntry {
                    name: focused_entry.to_string(),
                    is_dir,
                },
            ],
            list_state,
            input: String::new(),
            input_mode: false,
            cmd_list: Vec::new(),
            work_list: vec![cwd.to_path_buf()],
            work_idx: 0,
            list_area: None,
            last_click: None,
            registered_paths: Vec::new(),
            confirm_delete: false,
            confirm_target: None,
        }
    }

    fn write_capture_script(dir: &Path) -> (PathBuf, PathBuf) {
        let script_path = dir.join("capture_target.sh");
        let out_path = dir.join("captured_target.txt");
        let script = format!(
            "#!/bin/sh\nprintf '%s' \"$1\" > \"{}\"\n",
            out_path.to_string_lossy()
        );
        fs::write(&script_path, script).expect("write capture script");
        (script_path, out_path)
    }

    #[test]
    fn test_main_e_opens_selected_file() {
        let dir = test_dir("open_file");
        let target = dir.join("sample.txt");
        fs::write(&target, "x").expect("write sample file");

        let (script_path, out_path) = write_capture_script(&dir);
        let mut ctx = make_ctx(format!("sh {}", script_path.to_string_lossy()));
        let mut state = make_state(&dir, "sample.txt", false);

        let _ = state.on_key(&mut ctx, KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE));

        let captured = fs::read_to_string(out_path).expect("read captured target");
        assert_eq!(captured, target.to_string_lossy());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_main_e_opens_selected_directory() {
        let dir = test_dir("open_dir");
        let target = dir.join("folder_a");
        fs::create_dir_all(&target).expect("create target directory");

        let (script_path, out_path) = write_capture_script(&dir);
        let mut ctx = make_ctx(format!("sh {}", script_path.to_string_lossy()));
        let mut state = make_state(&dir, "folder_a", true);

        let _ = state.on_key(&mut ctx, KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE));

        let captured = fs::read_to_string(out_path).expect("read captured target");
        assert_eq!(captured, target.to_string_lossy());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_main_e_on_parent_entry_opens_current_directory() {
        let dir = test_dir("open_current");
        let (script_path, out_path) = write_capture_script(&dir);
        let mut ctx = make_ctx(format!("sh {}", script_path.to_string_lossy()));

        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut state = MainState {
            cwd: dir.clone(),
            items: vec![DirEntry {
                name: "..".to_string(),
                is_dir: true,
            }],
            list_state,
            input: String::new(),
            input_mode: false,
            cmd_list: Vec::new(),
            work_list: vec![dir.clone()],
            work_idx: 0,
            list_area: None,
            last_click: None,
            registered_paths: Vec::new(),
            confirm_delete: false,
            confirm_target: None,
        };

        let _ = state.on_key(&mut ctx, KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE));

        let captured = fs::read_to_string(out_path).expect("read captured target");
        assert_eq!(captured, dir.to_string_lossy());

        let _ = fs::remove_dir_all(dir);
    }
}
