use std::path::{PathBuf};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::app::{open_in_editor, AppContext};
use crate::system::expand_tilde;
use crate::ui::common::{Action, Screen, INPUT_PREFIX, mouse_pos, is_double_click};
use crate::ui::main_ui::{MainState};
use crate::ui::goto_ctrl::{GotoCtrl, GotoItem};

pub struct GotoState {
    pub ctrl: GotoCtrl,
    pub list_state: ListState,
    pub list_area: Option<Rect>,
    pub last_click: Option<(Instant, usize)>,
}

impl GotoState {
    pub fn new(ctx: &AppContext) -> anyhow::Result<Self> {
        Self::with_filter(ctx, "")
    }

    pub fn with_filter(ctx: &AppContext, filter: &str) -> anyhow::Result<Self> {
        let mut ctrl = GotoCtrl::new(ctx)?;
        ctrl.filter = filter.to_string();
        let mut list_state = ListState::default();
        list_state.select(Some(ctrl.selected_idx));
        Ok(Self {
            ctrl,
            list_state,
            list_area: None,
            last_click: None,
        })
    }

    pub fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(0),    // List
                Constraint::Length(2), // Filter
            ])
            .split(f.size());

        let title_widget = Paragraph::new(Line::from(Span::styled(
            " >> Goto (Repo / Local Dir / Local File)",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        f.render_widget(title_widget, layout[0]);

        let filtered = self.ctrl.filtered_items();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|item| {
                let (icon, label, style) = match item {
                    GotoItem::Repo(reg) => (
                        "[R] ",
                        reg.path.as_str(),
                        Style::default().fg(Color::White),
                    ),
                    GotoItem::LocalDir(dir) => (
                        "[D] ",
                        dir.name.as_str(),
                        Style::default().fg(Color::Green),
                    ),
                    GotoItem::LocalFile(file) => (
                        "[F] ",
                        file.name.as_str(),
                        Style::default().fg(Color::Blue),
                    ),
                };
                ListItem::new(format!("{}{}", icon, label)).style(style)
            })
            .collect();
        
        self.list_state.select(Some(self.ctrl.selected_idx));
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

        let input = Paragraph::new(format!("{}{}", INPUT_PREFIX, self.ctrl.filter))
            .block(Block::default().title("Filter"));
        f.render_widget(input, layout[2]);
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('Q') | KeyCode::Esc | KeyCode::Left => {
                return Ok(Action::Switch(Screen::Main(Box::new(MainState::new(ctx)?))));
            }
            KeyCode::Char('j') | KeyCode::Down => self.ctrl.next(),
            KeyCode::Char('k') | KeyCode::Up => self.ctrl.prev(),
            KeyCode::Enter => {
                if let Some(item) = self.ctrl.focus_item() {
                    match item {
                        GotoItem::Repo(reg) => {
                            let path = PathBuf::from(expand_tilde(&reg.path));
                            std::env::set_current_dir(path)?;
                            return Ok(Action::Switch(Screen::Main(Box::new(MainState::new(ctx)?))));
                        }
                        GotoItem::LocalDir(dir) => {
                            let path = std::env::current_dir()?.join(dir.name);
                            std::env::set_current_dir(path)?;
                            return Ok(Action::Switch(Screen::Main(Box::new(MainState::new(ctx)?))));
                        }
                        GotoItem::LocalFile(file) => {
                            let path = std::env::current_dir()?.join(file.name);
                            open_in_editor(&ctx.config.edit_app, path.to_string_lossy().as_ref());
                        }
                    }
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
                    self.ctrl.filter.push(c);
                }
            }
            KeyCode::Backspace => {
                self.ctrl.filter.pop();
            }
            _ => {}
        }
        Ok(Action::None)
    }

    pub fn on_mouse(&mut self, ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) => {
                        let inner = area.inner(&ratatui::layout::Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            let filtered = self.ctrl.filtered_items();
                            if idx < filtered.len() {
                                self.ctrl.set_selected(idx);
                                if is_double_click(&mut self.last_click, idx) {
                                    let item = &filtered[idx];
                                    match item {
                                        GotoItem::Repo(reg) => {
                                            let path = PathBuf::from(expand_tilde(&reg.path));
                                            std::env::set_current_dir(path)?;
                                            return Ok(Action::Switch(Screen::Main(Box::new(MainState::new(ctx)?))));
                                        }
                                        GotoItem::LocalDir(dir) => {
                                            let path = std::env::current_dir()?.join(&dir.name);
                                            std::env::set_current_dir(path)?;
                                            return Ok(Action::Switch(Screen::Main(Box::new(MainState::new(ctx)?))));
                                        }
                                        GotoItem::LocalFile(file) => {
                                            let path = std::env::current_dir()?.join(&file.name);
                                            open_in_editor(&ctx.config.edit_app, path.to_string_lossy().as_ref());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        self.ctrl.next();
                    }
                    MouseEventKind::ScrollUp => {
                        self.ctrl.prev();
                    }
                    _ => {}
                }
            }
        }
        Ok(Action::None)
    }
}

impl crate::ui::common::ScreenState for GotoState {
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