use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::{KeyEvent, MouseEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
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
    GitHistory(Box<dyn ScreenState>),
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
        if line.starts_with("diff --git") {
            out.push(Line::from(Span::styled(
                line.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if line.starts_with("index ") {
            out.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }
        if line.starts_with("--- ") {
            out.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::Red),
            )));
            continue;
        }
        if line.starts_with("+++ ") {
            out.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::Green),
            )));
            continue;
        }
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
        if line.starts_with('+') {
            out.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::Green),
            )));
            continue;
        }
        if line.starts_with('-') {
            out.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::Red),
            )));
            continue;
        }

        out.push(Line::from(Span::styled(line.clone(), Style::default())));
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

    #[test]
    fn test_format_diff_lines_git_diff_headers() {
        let lines = vec![
            "diff --git a/lib/main.dart b/lib/main.dart".to_string(),
            "index 1111111..2222222 100644".to_string(),
            "--- a/lib/main.dart".to_string(),
            "+++ b/lib/main.dart".to_string(),
        ];
        let out = format_diff_lines(&lines, 80);
        assert_eq!(out[0].spans[0].style.add_modifier, Modifier::BOLD);
        assert_eq!(out[1].spans[0].style.fg, Some(Color::DarkGray));
        assert_eq!(out[2].spans[0].style.fg, Some(Color::Red));
        assert_eq!(out[3].spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_format_diff_lines_git_diff_hunk_and_changes() {
        let lines = vec![
            "@@ -1,2 +1,2 @@".to_string(),
            "-old line".to_string(),
            "+new line".to_string(),
            " context".to_string(),
        ];
        let out = format_diff_lines(&lines, 80);
        assert_eq!(out[0].spans[0].style.fg, Some(Color::Cyan));
        assert_eq!(out[1].spans[0].style.fg, Some(Color::Red));
        assert_eq!(out[2].spans[0].style.fg, Some(Color::Green));
        assert_eq!(out[3].spans[0].style.fg, None);
    }
    
