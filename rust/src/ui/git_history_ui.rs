use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::app::AppContext;
use crate::ui::common::{format_diff_lines, is_double_click, mouse_pos, Action, Screen};
use crate::ui::git_history_ctrl::GitHistoryCtrl;

pub struct GitHistoryState {
    pub ctrl: GitHistoryCtrl,
    pub list_state: ListState,
    pub input_mode: bool,
    pub list_area: Option<Rect>,
    pub detail_area: Option<Rect>,
    pub last_click: Option<(Instant, usize)>,
}

impl GitHistoryState {
    pub fn new(ctx: &AppContext) -> anyhow::Result<Self> {
        let ctrl = GitHistoryCtrl::new(ctx)?;
        let mut list_state = ListState::default();
        list_state.select(Some(ctrl.selected_idx));
        Ok(Self {
            ctrl,
            list_state,
            input_mode: false,
            list_area: None,
            detail_area: None,
            last_click: None,
        })
    }

    pub fn render(&mut self, f: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(f.size());

        let input_title = if self.input_mode {
            "Filter (author, subject) [input]"
        } else {
            "Filter (author, subject)"
        };
        let filter =
            Paragraph::new(self.ctrl.filter.clone()).block(Block::default().title(input_title));
        f.render_widget(filter, layout[0]);
        if self.input_mode {
            let inner = layout[0].inner(&Margin {
                horizontal: 1,
                vertical: 1,
            });
            let cursor_x = inner
                .x
                .saturating_add(self.ctrl.filter.len() as u16)
                .min(inner.x + inner.width.saturating_sub(1));
            f.set_cursor(cursor_x, inner.y);
        }

        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(1)])
            .split(layout[1]);

        let items: Vec<ListItem> = if self.ctrl.filtered.is_empty() {
            vec![ListItem::new("< No commit >")]
        } else {
            self.ctrl
                .filtered
                .iter()
                .map(|c| ListItem::new(c.to_list_label()))
                .collect()
        };
        self.list_state.select(if self.ctrl.filtered.is_empty() {
            None
        } else {
            Some(self.ctrl.selected_idx)
        });
        let list = List::new(items)
            .block(Block::default().title("Commits"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, body[0], &mut self.list_state);
        self.list_area = Some(body[0]);

        let detail_lines = format_diff_lines(&self.ctrl.detail, body[1].width);
        let detail =
            Paragraph::new(Text::from(detail_lines)).block(Block::default().title("Detail"));
        f.render_widget(detail.scroll((self.ctrl.detail_scroll, 0)), body[1]);
        self.detail_area = Some(body[1]);
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        if self.input_mode {
            match key.code {
                KeyCode::Esc => self.input_mode = false,
                KeyCode::Enter => self.input_mode = false,
                KeyCode::Backspace => {
                    self.ctrl.filter.pop();
                    self.ctrl.apply_filter()?;
                }
                KeyCode::Char(c) => {
                    if !c.is_control() {
                        self.ctrl.filter.push(c);
                        self.ctrl.apply_filter()?;
                    }
                }
                _ => {}
            }
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left => {
                return Ok(Action::Switch(Screen::Main(Box::new(
                    crate::ui::main_ui::MainState::new(ctx)?,
                ))));
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                match crate::ui::git_stage_ui::GitStageState::new(ctx) {
                    Ok(state) => return Ok(Action::Switch(Screen::GitStage(Box::new(state)))),
                    Err(err) => return Ok(Action::Toast(err.to_string())),
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = true;
            }
            KeyCode::Char('j')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.ctrl.detail_scroll = self.ctrl.detail_scroll.saturating_add(3);
            }
            KeyCode::Char('k')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.ctrl.detail_scroll = self.ctrl.detail_scroll.saturating_sub(3);
            }
            KeyCode::Char('j') | KeyCode::Down => self.ctrl.next()?,
            KeyCode::Char('k') | KeyCode::Up => self.ctrl.prev()?,
            KeyCode::PageDown => self.ctrl.page_down()?,
            KeyCode::PageUp => self.ctrl.page_up()?,
            KeyCode::Char('g') => self.ctrl.set_selected(0)?,
            KeyCode::Char('G') => {
                let last = self.ctrl.filtered.len().saturating_sub(1);
                self.ctrl.set_selected(last)?;
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
                        let inner = area.inner(&Margin {
                            horizontal: 1,
                            vertical: 1,
                        });
                        if me.row >= inner.y && me.row < inner.y + inner.height {
                            let idx = (me.row - inner.y) as usize;
                            if idx < self.ctrl.filtered.len() {
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

        if let Some(area) = self.detail_area {
            if area.contains(mouse_pos(&me)) {
                if matches!(me.kind, MouseEventKind::ScrollDown) {
                    self.ctrl.detail_scroll = self.ctrl.detail_scroll.saturating_add(3);
                } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                    self.ctrl.detail_scroll = self.ctrl.detail_scroll.saturating_sub(3);
                }
            }
        }
        Ok(Action::None)
    }
}

impl crate::ui::common::ScreenState for GitHistoryState {
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
