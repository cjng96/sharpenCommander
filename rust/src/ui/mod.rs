pub mod common;
pub mod main_ui;
pub mod main_ctrl;
pub mod find_ui;
pub mod find_ctrl;
pub mod grep_ui;
pub mod grep_ctrl;
pub mod git_status_ui;
pub mod git_status_ctrl;
pub mod git_commit_ui;
pub mod git_commit_ctrl;
pub mod reg_list_ui;
pub mod reg_list_ctrl;
pub mod goto_ui;
pub mod goto_ctrl;
pub mod git_push_ui;

use std::io::{self, Stdout};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEvent, MouseEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Terminal;

use crate::app::AppContext;
use crate::ui::common::{Action, Screen, REDRAW_REQUEST};

pub use git_push_ui::git_push;

pub fn run(ctx: &mut AppContext) -> anyhow::Result<()> {
    let mut app = App::new(ctx)?;
    run_app(&mut app)
}

pub fn run_git_status(ctx: &mut AppContext) -> anyhow::Result<()> {
    let state = git_status_ui::GitStatusState::new(ctx)?;
    let mut app = App::new(ctx)?;
    app.screen = Screen::GitStatus(Box::new(state));
    run_app(&mut app)
}

pub fn run_find(ctx: &mut AppContext, args: &[String]) -> anyhow::Result<()> {
    let state = find_ui::FindState::from_args(ctx, args)?;
    let mut app = App::new(ctx)?;
    app.screen = Screen::Find(Box::new(state));
    run_app(&mut app)
}

pub fn run_grep(ctx: &mut AppContext, args: &[String]) -> anyhow::Result<()> {
    let state = grep_ui::GrepState::from_args(ctx, args)?;
    let mut app = App::new(ctx)?;
    app.screen = Screen::Grep(Box::new(state));
    run_app(&mut app)
}

pub fn run_goto(ctx: &mut AppContext, filter: &str) -> anyhow::Result<()> {
    let state = goto_ui::GotoState::with_filter(ctx, filter)?;
    let mut app = App::new(ctx)?;
    app.screen = Screen::Goto(Box::new(state));
    run_app(&mut app)
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

struct App<'a> {
    ctx: &'a mut AppContext,
    screen: Screen,
    toast: Option<(String, Instant)>,
}

impl<'a> App<'a> {
    fn new(ctx: &'a mut AppContext) -> anyhow::Result<Self> {
        let main = main_ui::MainState::new(ctx)?;
        Ok(Self {
            ctx,
            screen: Screen::Main(Box::new(main)),
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
                        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                            return Ok(());
                        }
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
                REDRAW_REQUEST.store(true, Ordering::SeqCst);
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
                REDRAW_REQUEST.store(true, Ordering::SeqCst);
                Ok(false)
            }
            Action::Toast(msg) => {
                self.set_toast(&msg, Duration::from_secs(2));
                Ok(false)
            }
        }
    }
}