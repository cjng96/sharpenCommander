use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Text};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::app::AppContext;
use crate::git;
use crate::system::{app_log, system, system_stream};
use crate::ui::common::{Action, Screen, mouse_pos, is_double_click, format_diff_lines, with_terminal_pause};
use crate::ui::git_commit_ctrl::GitCommitCtrl;

pub struct GitCommitState {
    pub ctrl: GitCommitCtrl,
    pub list_state: ListState,
    pub file_area: Option<Rect>,
    pub content_area: Option<Rect>,
    pub last_click: Option<(Instant, usize)>,
}

impl GitCommitState {
    pub fn new(ctx: &AppContext) -> anyhow::Result<Self> {
        let ctrl = GitCommitCtrl::new(ctx)?;
        let mut list_state = ListState::default();
        list_state.select(Some(ctrl.selected_idx));
        Ok(Self {
            ctrl,
            list_state,
            file_area: None,
            content_area: None,
            last_click: None,
        })
    }

    pub fn render(&mut self, f: &mut ratatui::Frame) {
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

        let input_title = if self.ctrl.input_mode {
            "Commit message (input)"
        } else {
            "Commit message"
        };
        let prompt = "Input commit message => ";
        let input = Paragraph::new(format!("{}{}", prompt, self.ctrl.message))
            .block(Block::default().title(input_title));
        f.render_widget(input, layout[0]);
        if self.ctrl.input_mode {
            let inner = layout[0].inner(&Margin {
                horizontal: 0,
                vertical: 1,
            });
            let cursor_x = inner
                .x
                .saturating_add(prompt.len() as u16)
                .saturating_add(self.ctrl.message.len() as u16)
                .min(inner.x + inner.width.saturating_sub(1));
            let cursor_y = inner.y.min(inner.y + inner.height.saturating_sub(1));
            f.set_cursor(cursor_x, cursor_y);
        }

        let file_items: Vec<ListItem> = self.ctrl.files.iter().map(|s| ListItem::new(s.as_str())).collect();
        self.list_state.select(Some(self.ctrl.selected_idx));
        let file_list = List::new(file_items)
            .block(Block::default().title("Files"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(file_list, layout[1], &mut self.list_state);
        self.file_area = Some(layout[1]);

        let commit_items: Vec<ListItem> = self.ctrl
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

        let diff_lines = format_diff_lines(&self.ctrl.content, layout[4].width);
        let view = Paragraph::new(Text::from(diff_lines)).block(Block::default().title("Diff"));
        f.render_widget(view.scroll((self.ctrl.content_scroll, 0)), layout[4]);
        self.content_area = Some(layout[4]);
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        if self.ctrl.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.ctrl.input_mode = false;
                }
                KeyCode::F(4) => {
                    return Ok(Action::Switch(Screen::GitStage(Box::new(crate::ui::git_stage_ui::GitStageState::new(ctx)?))));
                }
                KeyCode::Down => {
                    self.ctrl.next()?;
                }
                KeyCode::Up => {
                    self.ctrl.prev()?;
                }
                KeyCode::Enter => {
                    if self.ctrl.message.trim().is_empty() {
                        return Ok(Action::None);
                    }
                let status = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&self.ctrl.repo_root)
                    .arg("commit")
                    .arg("-m")
                    .arg(&self.ctrl.message)
                    .status()?;
                if !status.success() {
                    return Err(anyhow::anyhow!("git commit failed"));
                }
                return Ok(Action::Switch(Screen::Main(Box::new(crate::ui::main_ui::MainState::new(ctx)?))));
            }
                KeyCode::Backspace => {
                    self.ctrl.message.pop();
                }
                KeyCode::Char(c) => {
                    if !c.is_control() {
                        self.ctrl.message.push(c);
                    }
                }
                _ => {}
            }
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(Action::Switch(Screen::GitStage(Box::new(crate::ui::git_stage_ui::GitStageState::new(ctx)?))));
            }
            KeyCode::F(4) => {
                return Ok(Action::Switch(Screen::GitStage(Box::new(crate::ui::git_stage_ui::GitStageState::new(ctx)?))));
            }
            KeyCode::Char('i') => {
                self.ctrl.input_mode = true;
            }
            KeyCode::Char('j') | KeyCode::Down => self.ctrl.next()?,
            KeyCode::Char('k') | KeyCode::Up => self.ctrl.prev()?,
            KeyCode::Char('A') | KeyCode::Char('a') => {
                let name = self.ctrl.focus_file_name().unwrap_or_default();
                if !name.is_empty() {
                    system(&git::git_cmd_at(&self.ctrl.repo_root, &format!("add \"{}\"", name)))?;
                    self.ctrl = GitCommitCtrl::new(ctx)?;
                }
            }
            KeyCode::Char('R') => {
                let name = self.ctrl.focus_file_name().unwrap_or_default();
                if !name.is_empty() {
                    system(&git::git_cmd_at(&self.ctrl.repo_root, &format!("reset \"{}\"", name)))?;
                    self.ctrl = GitCommitCtrl::new(ctx)?;
                }
            }
            KeyCode::Char('T') => {
                with_terminal_pause(|| {
                    app_log("Running tig (GitCommit)");
                    let root = git::repo_root().unwrap_or_else(|_| std::path::PathBuf::from("."));
                    let old = std::env::current_dir()?;
                    let _ = std::env::set_current_dir(&root);
                    let res = system_stream("tig");
                    let _ = std::env::set_current_dir(old);
                    app_log(&format!("tig result: {:?}", res));
                    Ok(())
                })?;
            }
            _ => {}
        }
        Ok(Action::None)
    }

    pub fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.file_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.ctrl.files.len() {
                                self.ctrl.set_selected(idx)?;
                                let _ = is_double_click(&mut self.last_click, idx);
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        let _ = self.ctrl.next();
                    }
                    MouseEventKind::ScrollUp => {
                        let _ = self.ctrl.prev();
                    }
                    _ => {}
                }
            }
        }
        if let Some(area) = self.content_area {
            if area.contains(mouse_pos(&me)) {
                if matches!(me.kind, MouseEventKind::ScrollDown) {
                    self.ctrl.content_scroll = self.ctrl.content_scroll.saturating_add(3);
                } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                    self.ctrl.content_scroll = self.ctrl.content_scroll.saturating_sub(3);
                }
            }
        }
        Ok(Action::None)
    }
}

impl crate::ui::common::ScreenState for GitCommitState {
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