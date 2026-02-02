use std::path::Path;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, List, ListItem, ListState};

use crate::app::AppContext;
use crate::ui::common::{Action, Screen, mouse_pos, is_double_click};
use crate::ui::grep_ctrl::GrepCtrl;

pub struct GrepState {
    pub ctrl: GrepCtrl,
    pub list_state: ListState,
    pub list_area: Option<Rect>,
    pub last_click: Option<(Instant, usize)>,
}

impl GrepState {
    pub fn from_args(ctx: &AppContext, args: &[String]) -> anyhow::Result<Self> {
        let ctrl = GrepCtrl::from_args(ctx, args)?;
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
        let items: Vec<ListItem> = self.ctrl
            .lines
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();
        
        self.list_state.select(Some(self.ctrl.selected_idx));
        let list = List::new(items)
            .block(Block::default().title("Grep"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, f.size(), &mut self.list_state);
        self.list_area = Some(f.size());
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Action::Switch(Screen::Main(Box::new(crate::ui::main_ui::MainState::new(ctx)?))));
            }
            KeyCode::Char('j') | KeyCode::Down => self.ctrl.next(),
            KeyCode::Char('k') | KeyCode::Up => self.ctrl.prev(),
            KeyCode::Enter => {
                if let Some(line) = self.ctrl.focus_line() {
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

    pub fn on_mouse(&mut self, _ctx: &mut AppContext, me: MouseEvent) -> anyhow::Result<Action> {
        if let Some(area) = self.list_area {
            if area.contains(mouse_pos(&me)) {
                match me.kind {
                    MouseEventKind::Down(_) => {
                        let inner = area.inner(&Margin { horizontal: 1, vertical: 1 });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.ctrl.lines.len() {
                                self.ctrl.set_selected(idx);
                                if is_double_click(&mut self.last_click, idx) {
                                    if let Some(line) = self.ctrl.focus_line() {
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

impl crate::ui::common::ScreenState for GrepState {
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