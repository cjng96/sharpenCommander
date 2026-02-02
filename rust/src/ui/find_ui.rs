use std::path::Path;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::app::{open_in_editor, AppContext};
use crate::ui::common::{Action, Screen, mouse_pos, is_double_click};
use crate::ui::find_ctrl::FindCtrl;

pub struct FindState {
    pub ctrl: FindCtrl,
    pub list_state: ListState,
    pub list_area: Option<Rect>,
    pub content_area: Option<Rect>,
    pub last_click: Option<(Instant, usize)>,
}

impl FindState {
    pub fn from_args(ctx: &AppContext, args: &[String]) -> anyhow::Result<Self> {
        let ctrl = FindCtrl::from_args(ctx, args)?;
        let mut list_state = ListState::default();
        list_state.select(Some(ctrl.selected_idx));
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
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(f.size());
        let items: Vec<ListItem> = self.ctrl
            .files
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();
        
        self.list_state.select(Some(self.ctrl.selected_idx));
        let list = List::new(items)
            .block(Block::default().title("Find"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, layout[0], &mut self.list_state);
        self.list_area = Some(layout[0]);

        let text = Text::from(self.ctrl.content.join("\n"));
        let view = Paragraph::new(text)
            .block(Block::default().title("Content"));
        f.render_widget(view.scroll((self.ctrl.content_scroll, 0)), layout[1]);
        self.content_area = Some(layout[1]);
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Action::Switch(Screen::Main(Box::new(crate::ui::main_ui::MainState::new(ctx)?))));
            }
            KeyCode::Char('j') | KeyCode::Down => self.ctrl.next(),
            KeyCode::Char('k') | KeyCode::Up => self.ctrl.prev(),
            KeyCode::Enter => {
                if let Some(file) = self.ctrl.focus_file() {
                    let path = Path::new(&file);
                    if let Some(parent) = path.parent() {
                        ctx.save_path(parent.to_string_lossy().as_ref())?;
                    }
                }
            }
            KeyCode::Char('E') => {
                if let Some(file) = self.ctrl.focus_file() {
                    open_in_editor(&ctx.config.edit_app, &file);
                }
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
                            if idx < self.ctrl.files.len() {
                                self.ctrl.set_selected(idx);
                                if is_double_click(&mut self.last_click, idx) {
                                    if let Some(file) = self.ctrl.focus_file() {
                                        let path = Path::new(&file);
                                        if let Some(parent) = path.parent() {
                                            let _ = parent.to_string_lossy();
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

impl crate::ui::common::ScreenState for FindState {
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