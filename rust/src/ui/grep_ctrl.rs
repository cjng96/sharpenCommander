use std::cmp::min;
use crate::app::AppContext;
use crate::system::system_safe;
use crate::util::strip_ansi;

pub struct GrepCtrl {
    pub lines: Vec<String>,
    pub selected_idx: usize,
}

impl GrepCtrl {
    pub fn from_args(ctx: &AppContext, args: &[String]) -> anyhow::Result<Self> {
        let cmd = if args.len() > 1 {
            let rest = args[1..].join(" ");
            format!("{} --group --color {}", ctx.config.grep_app, rest)
        } else {
            format!("{} --group --color", ctx.config.grep_app)
        };
        let (out, _code) = system_safe(&cmd);
        let clean = strip_ansi(&out);
        let mut lines: Vec<String> = clean.lines().map(|l| l.to_string()).collect();
        if lines.is_empty() {
            lines.push("< No result >".to_string());
        }
        let ctrl = Self {
            lines,
            selected_idx: 0,
        };
        Ok(ctrl)
    }

    pub fn focus_line(&self) -> Option<String> {
        self.lines.get(self.selected_idx).cloned()
    }

    pub fn next(&mut self) {
        if !self.lines.is_empty() {
            self.selected_idx = min(self.selected_idx + 1, self.lines.len().saturating_sub(1));
        }
    }

    pub fn prev(&mut self) {
        self.selected_idx = self.selected_idx.saturating_sub(1);
    }

    pub fn set_selected(&mut self, idx: usize) {
        if idx < self.lines.len() {
            self.selected_idx = idx;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grep_ctrl_basic() {
        let lines = vec!["file1:1:match1".to_string(), "file2:10:match2".to_string()];
        let mut ctrl = GrepCtrl::with_lines(lines);
        
        assert_eq!(ctrl.lines.len(), 2);
        assert_eq!(ctrl.focus_line(), Some("file1:1:match1".to_string()));

        ctrl.next();
        assert_eq!(ctrl.selected_idx, 1);
        assert_eq!(ctrl.focus_line(), Some("file2:10:match2".to_string()));

        ctrl.prev();
        assert_eq!(ctrl.selected_idx, 0);
        
        ctrl.set_selected(1);
        assert_eq!(ctrl.selected_idx, 1);
    }
}

impl GrepCtrl {
    pub fn with_lines(lines: Vec<String>) -> Self {
        Self {
            lines,
            selected_idx: 0,
        }
    }
}
