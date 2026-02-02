use std::path::{Path, PathBuf};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph};

use crate::app::AppContext;
use crate::config::RegItem;
use crate::git::{PullStatus};
use crate::system::{app_log, system_stream};
use crate::ui::common::{Action, Screen, INPUT_PREFIX, mouse_pos, is_double_click, centered_rect, with_terminal_pause};
use crate::ui::reg_list_ctrl::{RegListCtrl, DetailMode};

pub struct RegListState {
    pub ctrl: RegListCtrl,
    pub list_state: ListState,
    pub list_area: Option<Rect>,
    pub log_area: Option<Rect>,
    pub confirm_delete: bool,
    pub confirm_target: Option<String>,
    pub last_click: Option<(Instant, usize)>,
}

impl RegListState {
    pub fn new(ctx: &mut AppContext) -> anyhow::Result<Self> {
        let ctrl = RegListCtrl::new(ctx)?;
        let mut list_state = ListState::default();
        list_state.select(Some(ctrl.selected_idx));
        Ok(Self {
            ctrl,
            list_state,
            list_area: None,
            log_area: None,
            confirm_delete: false,
            confirm_target: None,
            last_click: None,
        })
    }

    pub fn render(&mut self, f: &mut ratatui::Frame) {
        self.ctrl.drain_pull_events();
        self.ctrl.drain_status_events();
        self.ctrl.drain_detail();

        let header = if self.ctrl.pull_total > 0 {
            format!("Repo list - pull {}/{}", self.ctrl.pull_done, self.ctrl.pull_total)
        } else {
            "Repo list".to_string()
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(0),    // Body (List + Status)
                Constraint::Length(2), // Filter (Bottom)
            ])
            .split(f.size());

        let title_widget = Paragraph::new(Line::from(Span::styled(
            format!(" >> {}", header),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        f.render_widget(title_widget, layout[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(layout[1]);
        
        self.render_list("".to_string(), body[0], f);
        
        if self.ctrl.detail_mode == DetailMode::Log {
            self.render_log(body[1], f);
        } else {
            self.render_status(body[1], f);
        }

        let input_text = format!("{}{}", INPUT_PREFIX, self.ctrl.filter);
        let input = Paragraph::new(input_text).block(Block::default().title("Filter"));
        f.render_widget(input, layout[2]);

        if self.confirm_delete {
            let area = centered_rect(40, 7, f.size());
            f.render_widget(Clear, area); 
            
            let target = self.confirm_target.clone().unwrap_or_default();
            let text = vec![
                Line::from(vec![
                    Span::raw("Delete from list? "),
                    Span::styled(target, Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
                ]),
                Line::from(Span::styled("(y) Yes / (N) No", Style::default().fg(Color::DarkGray))),
            ];
            
            let block = Block::default()
                .title(" Confirmation ")
                .style(Style::default().bg(Color::Black).fg(Color::DarkGray));
                
            let p = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
                
            f.render_widget(p, area);
        }
    }

    pub fn on_key(&mut self, ctx: &mut AppContext, key: KeyEvent) -> anyhow::Result<Action> {
        if self.confirm_delete {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(path) = self.confirm_target.clone() {
                        ctx.reg_remove(&path)?;
                        self.ctrl.items.retain(|i| i.path != path);
                    }
                    self.confirm_delete = false;
                    self.confirm_target = None;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Enter => {
                    self.confirm_delete = false;
                    self.confirm_target = None;
                }
                _ => {}
            }
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Esc => {
                if !self.ctrl.filter.is_empty() {
                    self.ctrl.filter.clear();
                    self.ctrl.select_at(0);
                    return Ok(Action::None);
                }
                return Ok(Action::Switch(Screen::Main(Box::new(crate::ui::main_ui::MainState::new(ctx)?))));
            }
            KeyCode::Char('Q') | KeyCode::Left => {
                return Ok(Action::Switch(Screen::Main(Box::new(crate::ui::main_ui::MainState::new(ctx)?))));
            }
            KeyCode::Down => self.ctrl.next(),
            KeyCode::Up => self.ctrl.prev(),
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::ALT) => self.ctrl.next(),
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::ALT) => self.ctrl.prev(),
            KeyCode::Char('P') => {
                let targets: Vec<RegItem> = self.ctrl.items.iter().filter(|i| i.repo).cloned().collect();
                self.ctrl.start_pull(targets, ctx.config.is_pull_rebase);
            }
            KeyCode::Char('F') => {
                if let Some(item) = self.ctrl.focus_item() {
                    if item.repo {
                        self.ctrl.start_pull(vec![item], true); 
                    }
                }
            }
            KeyCode::Char('S') | KeyCode::Char('s') => {
                self.ctrl.detail_mode = DetailMode::Status;
                self.ctrl.fetch_detail();
            }
            KeyCode::Char('T') => {
                if let Some(item) = self.ctrl.focus_item() {
                    let target = item.path.clone();
                    with_terminal_pause(|| {
                        let old = std::env::current_dir()?;
                        if std::env::set_current_dir(&target).is_ok() {
                            app_log(&format!("Running tig in {}", target));
                            let res = system_stream("tig");
                            app_log(&format!("tig result: {:?}", res));
                            let _ = std::env::set_current_dir(old);
                        }
                        Ok(())
                    })?;
                }
            }
            KeyCode::Char('D') => {
                if let Some(item) = self.ctrl.focus_item() {
                    self.confirm_delete = true;
                    self.confirm_target = Some(item.path);
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.ctrl.focus_item() {
                    let path = if Path::new(&item.path).is_absolute() {
                        PathBuf::from(&item.path)
                    } else {
                        std::env::current_dir()?.join(&item.path)
                    };
                    if std::env::set_current_dir(&path).is_ok() {
                        return Ok(Action::Switch(Screen::Main(Box::new(crate::ui::main_ui::MainState::new(ctx)?))));
                    } else {
                        return Ok(Action::Toast(format!("Failed to enter {}", path.to_string_lossy())));
                    }
                }
            }
            KeyCode::PageDown => {
                self.ctrl.log_scroll = self.ctrl.log_scroll.saturating_add(5);
            }
            KeyCode::PageUp => {
                self.ctrl.log_scroll = self.ctrl.log_scroll.saturating_sub(5);
            }
            KeyCode::Delete => {
                if let Some(item) = self.ctrl.focus_item() {
                    ctx.reg_remove(&item.path)?;
                    self.ctrl.items.retain(|i| i.path != item.path);
                }
            }
            KeyCode::Backspace => {
                self.ctrl.filter.pop();
                self.ctrl.select_at(0);
            }
            KeyCode::Char(c) => {
                self.ctrl.filter.push(c);
                self.ctrl.select_at(0);
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
                            if idx < self.ctrl.filtered_items().len() {
                                self.ctrl.select_at(idx);
                                if self.ctrl.detail_mode != DetailMode::None {
                                    self.ctrl.fetch_detail();
                                }
                                if is_double_click(&mut self.last_click, idx) {
                                    if let Some(item) = self.ctrl.focus_item() {
                                        self.ctrl.detail_mode = DetailMode::Log;
                                        self.ctrl.log_path = Some(item.path);
                                        self.ctrl.log_scroll = 0;
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
        if let Some(area) = self.log_area {
            if area.contains(mouse_pos(&me)) {
                if matches!(me.kind, MouseEventKind::ScrollDown) {
                    self.ctrl.log_scroll = self.ctrl.log_scroll.saturating_add(3);
                } else if matches!(me.kind, MouseEventKind::ScrollUp) {
                    self.ctrl.log_scroll = self.ctrl.log_scroll.saturating_sub(3);
                }
            }
        }
        Ok(Action::None)
    }

    fn render_list(&mut self, _header: String, area: Rect, f: &mut ratatui::Frame) {
        let filtered = self.ctrl.filtered_items();
        let list_width = area.width.saturating_sub(4) as usize; // account for margins and selection symbol

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|i| {
                let status_info = self.ctrl.status_infos.get(&i.path);
                let display_name = i.display_name(status_info);
                
                let (pull_status_label, pull_msg, mut base_style) = if let Some(info) = self.ctrl.pull_infos.get(&i.path) {
                    let status = info.status.label().to_string();
                    let msg = match &info.status {
                        PullStatus::Done { code: _, message } => message.clone(),
                        _ => None,
                    };
                    let style = match &info.status {
                        PullStatus::Pending => Style::default().fg(Color::DarkGray),
                        PullStatus::Running => Style::default().fg(Color::Yellow),
                        PullStatus::Done { code, .. } => {
                            if *code == 0 {
                                Style::default().fg(Color::Green)
                            } else {
                                Style::default().fg(Color::Red)
                            }
                        }
                    };
                    (status, msg, style)
                } else {
                    let mut style = Style::default();
                    let is_subfolder = self.ctrl.items.iter().any(|j| {
                        j.repo && i.path.starts_with(&j.path) && i.path != j.path
                    });

                    if is_subfolder {
                        style = style.fg(Color::DarkGray);
                    } else if let Some(info) = status_info {
                        if info.dirty {
                            style = style.fg(Color::Green);
                        } else if info.ahead > 0 || info.behind > 0 {
                            style = style.fg(Color::Yellow);
                        }
                    }
                    
                    ("".to_string(), None, style)
                };
                
                // Set base_style based on status priority: Dirty (Green) > Ahead/Behind (Yellow) > Normal
                if let Some(info) = status_info {
                    if info.dirty {
                        base_style = base_style.fg(Color::Green);
                    } else if info.ahead > 0 || info.behind > 0 {
                        base_style = base_style.fg(Color::Yellow);
                    }
                }

                let mut left_spans = Vec::new();
                
                if display_name.contains('*') {
                    let parts: Vec<&str> = display_name.split('*').collect();
                    for (idx, part) in parts.iter().enumerate() {
                        left_spans.push(Span::styled(part.to_string(), base_style));
                        if idx < parts.len() - 1 {
                            left_spans.push(Span::styled("*", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                        }
                    }
                } else {
                    left_spans.push(Span::styled(display_name.clone(), base_style));
                }

                let mut right_spans = Vec::new();
                let mut right_text_len = 0;

                if !pull_status_label.is_empty() {
                    let status_part = if pull_status_label == "ERR" {
                        if let Some(msg) = pull_msg {
                            format!(" [{}] {}", pull_status_label, msg)
                        } else {
                            format!(" [{}]", pull_status_label)
                        }
                    } else {
                        format!(" [{}]", pull_status_label)
                    };
                    right_text_len += status_part.len();
                    right_spans.push(Span::styled(status_part, base_style));
                }

                let mut spans = left_spans;
                let left_len = display_name.chars().count();
                let padding_len = list_width.saturating_sub(left_len + right_text_len);
                
                if padding_len > 0 {
                    spans.push(Span::raw(" ".repeat(padding_len)));
                } else {
                    spans.push(Span::raw(" "));
                }
                
                spans.extend(right_spans);

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().style(Style::default().bg(Color::Black)))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(64, 64, 64))
                    .fg(Color::Cyan),
            )
            .highlight_symbol(">>> ");

        self.list_state.select(Some(self.ctrl.selected_idx));
        f.render_widget(Clear, area);
        f.render_stateful_widget(list, area, &mut self.list_state);
        self.list_area = Some(area);
    }

    fn render_log(&mut self, area: Rect, f: &mut ratatui::Frame) {
        f.render_widget(Clear, area);

        let title = self.ctrl
            .log_path
            .clone()
            .unwrap_or_else(|| "< no selection >".to_string());
        let lines = if let Some(path) = &self.ctrl.log_path {
            self.ctrl.pull_infos
                .get(path)
                .map(|info| info.log.clone())
                .unwrap_or_else(|| vec!["< No log >".to_string()])
        } else {
            vec!["< No log >".to_string()]
        };
        let text = Text::from(lines.join("\n"));
        let view = Paragraph::new(text)
            .block(Block::default().title(title).style(Style::default().bg(Color::Black)))
            .style(Style::default().bg(Color::Black))
            .scroll((self.ctrl.log_scroll, 0));
        
        f.render_widget(view, area);
        self.log_area = Some(area);
    }

    fn render_status(&mut self, area: Rect, f: &mut ratatui::Frame) {
        let title = format!("Status - {}", self.ctrl.log_path.clone().unwrap_or_default());
        let mut styled_lines = Vec::new();

        let width = area.width as usize;
        
        for line in &self.ctrl.status_lines {
            // let style = if line.starts_with("On branch") {
            //     Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            // } else if line.contains("Your branch is") {
            //      Style::default().fg(Color::Yellow)
            // } else if line.starts_with("Untracked files:") || line.contains("Changes not staged for commit:") {
            //     Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            // } else if line.contains("Changes to be committed:") {
            //     Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            // } else if line.trim().starts_with("modified:") {
            //      Style::default().fg(Color::Red)
            // } else if line.trim().starts_with("deleted:") {
            //      Style::default().fg(Color::Red)
            // } else if line.trim().starts_with("new file:") {
            //      Style::default().fg(Color::Green)
            // } else {
            //     Style::default()
            // };
            let style = Style::default();

            // 일단 이렇게라도 하자
            // let padded_line = if line.chars().count() < width {
            //     let mut s = line.clone();
            //     let pad_len = width.saturating_sub(line.chars().count());
            //     // Add pad_len spaces at the end
            //     for _ in 0..pad_len {
            //         s.push(' ');
            //     }
            //     s
            // } else {
            //     line.clone()
            // };
            
            styled_lines.push(Line::from(Span::styled(line.clone(), style)));
        }

        // 빈 공간을 공백 라인으로 채운다 (세로 채움)
        // let styled_lines_len = styled_lines.len();
        // let area_height = area.height as usize;
        // if styled_lines_len < area_height {
        //     for _ in 0..(area_height - styled_lines_len) {
        //         styled_lines.push(Line::from(Span::styled(
        //             " ".repeat(width),
        //             Style::default(),
        //         )));
        //     }
        // }

        let view = Paragraph::new(Text::from(styled_lines))
            .block(Block::default().title(title).style(Style::default().bg(Color::Black)))
            // .style(Style::default().bg(Color::Black))
            .scroll((self.ctrl.log_scroll, 0));
        
        f.render_widget(Clear, area);
        f.render_widget(view, area);
        self.log_area = Some(area);
    }
}

impl crate::ui::common::ScreenState for RegListState {
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
