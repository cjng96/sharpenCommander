use std::cmp::min;
use crate::app::AppContext;
use crate::system::system_safe;
use crate::util::file_size;

pub struct FindCtrl {
    pub files: Vec<String>,
    pub selected_idx: usize,
    pub content: Vec<String>,
    pub content_scroll: u16,
}

impl FindCtrl {
    pub fn from_args(_ctx: &AppContext, args: &[String]) -> anyhow::Result<Self> {
        let cmd = if args.len() > 1 {
            let rest = args[1..].join(" ");
            if rest.trim_start().starts_with('-') {
                format!("find . {}", rest)
            } else {
                format!("find {}", rest)
            }
        } else {
            "find .".to_string()
        };
        let (out, _code) = system_safe(&cmd);
        let mut files: Vec<String> = out.lines().map(|l| l.to_string()).collect();
        files.retain(|s| !s.trim().is_empty());
        let mut ctrl = Self {
            files,
            selected_idx: 0,
            content: vec!["< Nothing to display >".to_string()],
            content_scroll: 0,
        };
        ctrl.load_content();
        Ok(ctrl)
    }

    pub fn focus_file(&self) -> Option<String> {
        self.files.get(self.selected_idx).cloned()
    }

    pub fn next(&mut self) {
        if !self.files.is_empty() {
            self.selected_idx = min(self.selected_idx + 1, self.files.len().saturating_sub(1));
            self.load_content();
        }
    }

    pub fn prev(&mut self) {
        self.selected_idx = self.selected_idx.saturating_sub(1);
        self.load_content();
    }

    pub fn set_selected(&mut self, idx: usize) {
        if idx < self.files.len() {
            self.selected_idx = idx;
            self.load_content();
        }
    }

    pub fn load_content(&mut self) {
        if let Some(file) = self.focus_file() {
            let text = std::fs::read_to_string(&file)
                .unwrap_or_else(|_| format!("No utf8 file[size:{}]", file_size(&file)));
            self.content = text.replace('\t', "    ").lines().map(|s| s.to_string()).collect();
        } else {
            self.content = vec!["< Nothing to display >".to_string()];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::ui::common::TestEnv;

    #[test]
    fn test_find_ctrl_basic() {
        let env = TestEnv::setup("test_find_ctrl");
        let p1 = env.root.join("test1.txt");
        let p2 = env.root.join("test2.txt");
        fs::write(&p1, "line1\nline2").unwrap();
        fs::write(&p2, "data").unwrap();
        
        // Use absolute paths to be safe with parallel tests
        let mut ctrl = FindCtrl::with_files(vec![
            p1.to_string_lossy().to_string(),
            p2.to_string_lossy().to_string(),
        ]);
        
        assert_eq!(ctrl.files.len(), 2);
        assert!(ctrl.content.iter().any(|l| l.contains("line1")));

        ctrl.next();
        assert_eq!(ctrl.selected_idx, 1);
        assert!(ctrl.content.iter().any(|l| l.contains("data")));

        ctrl.prev();
        assert_eq!(ctrl.selected_idx, 0);
        
        ctrl.set_selected(1);
        assert_eq!(ctrl.selected_idx, 1);
    }
}

impl FindCtrl {
    pub fn with_files(files: Vec<String>) -> Self {
        let mut ctrl = Self {
            files,
            selected_idx: 0,
            content: vec![],
            content_scroll: 0,
        };
        ctrl.load_content();
        ctrl
    }
}
