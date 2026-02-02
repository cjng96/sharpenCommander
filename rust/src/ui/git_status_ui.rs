use std::path::{Path};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::app::{open_in_editor, AppContext};
use crate::git::{self, GitItemKind};
use crate::system::{app_log, system, system_stream};
use crate::ui::common::{Action, Screen, mouse_pos, is_double_click, format_diff_lines, with_terminal_pause};
use crate::ui::git_status_ctrl::GitStatusCtrl;

pub struct GitStatusState {
    pub ctrl: GitStatusCtrl,
    pub list_state: ListState,
    pub list_area: Option<Rect>,
    pub content_area: Option<Rect>,
    pub last_click: Option<(Instant, usize)>,
}

impl GitStatusState {
    pub fn new(ctx: &AppContext) -> anyhow::Result<Self> {
        let ctrl = GitStatusCtrl::new(ctx)?;
        let mut list_state = ListState::default();
        list_state.select(ctrl.selected_idx);
        Ok(Self {
            ctrl,
            list_state,
            list_area: None,
            content_area: None,
            last_click: None,
        })
    }

    pub fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(f.size());
        let items: Vec<ListItem> = self.ctrl
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
        
        self.list_state.select(self.ctrl.selected_idx);
        let list = List::new(items)
            .block(Block::default().title("Git status"))
            .highlight_style(Style::default().add_modifier(ratatui::style::Modifier::REVERSED));
        f.render_stateful_widget(list, layout[0], &mut self.list_state);
        self.list_area = Some(layout[0]);

        let diff_lines = format_diff_lines(&self.ctrl.content, layout[1].width);
        let text = Text::from(diff_lines);
        let view = Paragraph::new(text)
            .block(Block::default().title("Diff"));
        f.render_widget(view.scroll((self.ctrl.content_scroll, 0)), layout[1]);
        self.content_area = Some(layout[1]);
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Action::Switch(Screen::Main(Box::new(crate::ui::main_ui::MainState::new(ctx)?))));
            }
            KeyCode::Char('j') | KeyCode::Down => self.ctrl.next()?,
            KeyCode::Char('k') | KeyCode::Up => self.ctrl.prev()?,
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if let Some(name) = self.ctrl.focus_file_name() {
                    system(&format!("git add \"{}\"", name))?;
                    self.ctrl.refresh()?;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Some(name) = self.ctrl.focus_file_name() {
                    system(&format!("git reset \"{}\"", name))?;
                    self.ctrl.refresh()?;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(idx) = self.ctrl.selected_idx {
                    if let Some(item) = self.ctrl.items.get(idx) {
                        if item.kind == GitItemKind::Entry {
                            if let Some(name) = &item.path {
                                let status = item.status.as_deref().unwrap_or("");
                                let mut msg = format!("Reverted: {}", name);
                                if status == "?" {
                                    let path = Path::new(name);
                                    if path.exists() {
                                        if path.is_dir() {
                                            let _ = std::fs::remove_dir_all(path);
                                        } else {
                                            let _ = std::fs::remove_file(path);
                                        }
                                        msg = format!("Deleted: {}", name);
                                    }
                                } else {
                                    if let Err(_) = system(&format!("git checkout HEAD -- \"{}\"", name)) {
                                        let _ = system(&format!("git reset HEAD \"{}\"", name));
                                        let path = Path::new(name);
                                        if path.exists() {
                                            if path.is_dir() {
                                                let _ = std::fs::remove_dir_all(path);
                                            } else {
                                                let _ = std::fs::remove_file(path);
                                            }
                                            msg = format!("Deleted (Staged New): {}", name);
                                        }
                                    }
                                }
                                self.ctrl.refresh()?;
                                return Ok(Action::Toast(msg));
                            }
                        }
                    }
                }
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                let target = self.ctrl.selected_idx.and_then(|idx| {
                    self.ctrl.items.get(idx).and_then(|item| {
                        if item.kind == GitItemKind::Entry {
                            item.path.as_ref().map(|p| (p.clone(), item.status.clone()))
                        } else {
                            None
                        }
                    })
                });

                if let Some((name, status)) = target {
                    let status_str = status.as_deref().unwrap_or("");
                    if status_str == "?" {
                        git::add_to_gitignore(&name)?;
                        self.ctrl.refresh()?;
                        return Ok(Action::Toast(format!("Added to .gitignore: {}", name)));
                    } else {
                        return Ok(Action::Toast(format!("Warning: {} is not untracked", name)));
                    }
                }
            }
            KeyCode::Char('C') => {
                match crate::ui::git_commit_ui::GitCommitState::new(ctx) {
                    Ok(state) => return Ok(Action::Switch(Screen::GitCommit(Box::new(state)))),
                    Err(err) => {
                        app_log(&format!("Key C (GitStatus) error: {}", err));
                        return Ok(Action::Toast(err.to_string()));
                    }
                }
            }
            KeyCode::Char('E') => {
                if let Some(name) = self.ctrl.focus_file_name() {
                    open_in_editor(&ctx.config.edit_app, &name);
                }
            }
            KeyCode::Char('T') => {
                with_terminal_pause(|| {
                    app_log("Running tig (GitStatus)");
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
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.ctrl.items.len() {
                                if self.ctrl.items[idx].kind == GitItemKind::Entry {
                                    self.ctrl.set_selected(idx)?;
                                    let _ = is_double_click(&mut self.last_click, idx);
                                }
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

impl crate::ui::common::ScreenState for GitStatusState {
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