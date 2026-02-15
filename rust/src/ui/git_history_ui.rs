use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::app::AppContext;
use crate::ui::common::{format_diff_lines, is_double_click, mouse_pos, Action, Screen};
use crate::ui::git_history_ctrl::GitHistoryCtrl;

const SECTION_TITLE_BG: Color = Color::DarkGray;
const SECTION_TITLE_FG: Color = Color::White;
const FILTER_INPUT_HEIGHT: u16 = 1;

fn section_title_line(label: &str) -> Line<'static> {
    Line::styled(
        format!(" {} ", label),
        Style::default().bg(SECTION_TITLE_BG).fg(SECTION_TITLE_FG),
    )
}

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
            .constraints([
                Constraint::Length(1),
                Constraint::Length(FILTER_INPUT_HEIGHT),
                Constraint::Length(1),
                Constraint::Length(12),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(f.size());

        let filter_title = if self.input_mode {
            "Filter (author, subject) [input]"
        } else {
            "Filter (author, subject)"
        };
        f.render_widget(
            Paragraph::new(section_title_line(filter_title))
                .style(Style::default().bg(SECTION_TITLE_BG).fg(SECTION_TITLE_FG)),
            layout[0],
        );
        let filter = Paragraph::new(self.ctrl.filter.clone()).block(Block::default());
        f.render_widget(filter, layout[1]);
        if self.input_mode {
            let cursor_x = layout[1]
                .x
                .saturating_add(self.ctrl.filter.len() as u16)
                .min(layout[1].x + layout[1].width.saturating_sub(1));
            f.set_cursor(cursor_x, layout[1].y);
        }

        f.render_widget(
            Paragraph::new(section_title_line("Commits"))
                .style(Style::default().bg(SECTION_TITLE_BG).fg(SECTION_TITLE_FG)),
            layout[2],
        );

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
            .block(Block::default())
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, layout[3], &mut self.list_state);
        self.list_area = Some(layout[3]);

        f.render_widget(
            Paragraph::new(section_title_line("Detail"))
                .style(Style::default().bg(SECTION_TITLE_BG).fg(SECTION_TITLE_FG)),
            layout[4],
        );

        let detail_lines = format_diff_lines(&self.ctrl.detail, layout[5].width);
        let detail = Paragraph::new(Text::from(detail_lines)).block(Block::default());
        f.render_widget(detail.scroll((self.ctrl.detail_scroll, 0)), layout[5]);
        self.detail_area = Some(layout[5]);
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
                        let inner = area;
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

#[cfg(test)]
mod tests {
    use super::{section_title_line, FILTER_INPUT_HEIGHT, SECTION_TITLE_BG, SECTION_TITLE_FG};

    #[test]
    fn test_section_title_line_uses_colors() {
        let line = section_title_line("Filter");
        assert_eq!(line.style.bg, Some(SECTION_TITLE_BG));
        assert_eq!(line.style.fg, Some(SECTION_TITLE_FG));
    }

    #[test]
    fn test_filter_input_height_is_one_line() {
        assert_eq!(FILTER_INPUT_HEIGHT, 1);
    }
}
