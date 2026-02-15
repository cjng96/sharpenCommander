use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::{KeyEvent, MouseEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::app::AppContext;

pub static REDRAW_REQUEST: AtomicBool = AtomicBool::new(false);
pub const INPUT_PREFIX: &str = "$ ";

pub enum Action {
    None,
    Exit,
    Switch(Screen),
    Toast(String),
}

pub enum Screen {
    Main(Box<dyn ScreenState>),
    Find(Box<dyn ScreenState>),
    Grep(Box<dyn ScreenState>),
    GitStage(Box<dyn ScreenState>),
    GitCommit(Box<dyn ScreenState>),
    RegList(Box<dyn ScreenState>),
    Goto(Box<dyn ScreenState>),
}

pub trait ScreenState {
    fn render(&mut self, f: &mut ratatui::Frame);
    fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action>;
    fn on_mouse(&mut self, ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action>;
}

pub fn format_diff_lines(lines: &[String], width: u16) -> Vec<Line<'static>> {
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

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn mouse_pos(me: &MouseEvent) -> Position {
    Position::new(me.column, me.row)
}

pub fn is_double_click(last: &mut Option<(Instant, usize)>, idx: usize) -> bool {
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

pub fn with_terminal_pause<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    use std::io::Write;
    disable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    io::stdout().flush()?;
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
    
    #[cfg(test)]
    pub struct TestEnv {
        pub root: std::path::PathBuf,
        pub old_cwd: std::path::PathBuf,
    }
    
    #[cfg(test)]
    impl TestEnv {
        pub fn setup(prefix: &str) -> Self {
            use std::env;
            use std::fs;
            let old_cwd = env::current_dir().unwrap();
            let mut root = old_cwd.clone();
            if !root.ends_with("rust") && root.join("rust").is_dir() {
                root.push("rust");
            }
            root.push("test-data");
            root.push(format!("{}_{}", prefix, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
            
            fs::create_dir_all(&root).unwrap();
            env::set_current_dir(&root).unwrap();
            
            Self { root, old_cwd }
        }
    }
    
    #[cfg(test)]
    impl Drop for TestEnv {
        fn drop(&mut self) {
            use std::env;
            use std::fs;
            env::set_current_dir(&self.old_cwd).unwrap();
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    