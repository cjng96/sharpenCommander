use std::path::{Path};
use crate::app::AppContext;
use crate::git::{self, GitItem, GitItemKind};
use crate::system::{system};
use crate::util::{file_size, strip_ansi};

pub struct GitStageCtrl {
    pub items: Vec<GitItem>,
    pub selected_idx: Option<usize>,
    pub content: Vec<String>,
    pub content_scroll: u16,
}

impl GitStageCtrl {
    pub fn new(_ctx: &AppContext) -> anyhow::Result<Self> {
        let items = git::build_git_items()?;
        Self::with_items(items)
    }

    pub fn with_items(items: Vec<GitItem>) -> anyhow::Result<Self> {
        if items.iter().all(|i| i.kind != GitItemKind::Entry) {
            return Err(anyhow::anyhow!("No modified or untracked files"));
        }
        let first_selectable = items.iter().position(|item| item.kind == GitItemKind::Entry);
        let mut ctrl = Self {
            items,
            selected_idx: first_selectable,
            content: vec!["< Nothing to display >".to_string()],
            content_scroll: 0,
        };
        let _ = ctrl.load_content();
        Ok(ctrl)
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        self.items = git::build_git_items()?;
        self.selected_idx = self.first_selectable();
        self.load_content()?;
        Ok(())
    }

    pub fn focus_file_name(&self) -> Option<String> {
        let idx = self.selected_idx?;
        let item = self.items.get(idx)?;
        if item.kind != GitItemKind::Entry {
            return None;
        }
        item.path.clone()
    }

    pub fn next(&mut self) -> anyhow::Result<()> {
        if let Some(i) = self.next_selectable(1) {
            self.selected_idx = Some(i);
            self.content_scroll = 0;
            self.load_content()?;
        }
        Ok(())
    }

    pub fn prev(&mut self) -> anyhow::Result<()> {
        if let Some(i) = self.prev_selectable(1) {
            self.selected_idx = Some(i);
            self.content_scroll = 0;
            self.load_content()?;
        }
        Ok(())
    }

    pub fn set_selected(&mut self, idx: usize) -> anyhow::Result<()> {
        if idx < self.items.len() && self.items[idx].kind == GitItemKind::Entry {
            self.selected_idx = Some(idx);
            self.load_content()?;
        }
        Ok(())
    }

    pub fn load_content(&mut self) -> anyhow::Result<()> {
        if let Some(name) = self.focus_file_name() {
            let status = self.selected_idx
                .and_then(|i| self.items.get(i))
                .and_then(|x| x.status.clone())
                .unwrap_or_default();
            let out_res = if Path::new(&name).is_dir() {
                Ok(format!("{} is folder", name))
            } else if status == "?" {
                Ok(std::fs::read_to_string(&name)
                    .unwrap_or_else(|_| format!("No utf8 file[size:{}]", file_size(&name))))
            } else if status == "s" {
                system(&format!("git diff --color --staged \"{}\"", name))
            } else {
                system(&format!("git diff --color \"{}\"", name))
            };
            
            let out = match out_res {
                Ok(o) => o,
                Err(e) => format!("Error loading content: {}", e),
            };
            self.content = strip_ansi(&out).replace('\t', "    ").lines().map(|s| s.to_string()).collect();
        } else {
            self.content = vec!["< Nothing to display >".to_string()];
        }
        Ok(())
    }

    fn first_selectable(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.kind == GitItemKind::Entry)
    }

    fn next_selectable(&self, step: usize) -> Option<usize> {
        let start = self.selected_idx.unwrap_or(0);
        let mut idx = start + step;
        while idx < self.items.len() {
            if self.items[idx].kind == GitItemKind::Entry {
                return Some(idx);
            }
            idx += 1;
        }
        None
    }

    fn prev_selectable(&self, step: usize) -> Option<usize> {
        let mut idx = self.selected_idx.unwrap_or(0);
        for _ in 0..=step {
            if idx == 0 {
                break;
            }
            idx -= 1;
            if self.items[idx].kind == GitItemKind::Entry {
                return Some(idx);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{GitItem, GitItemKind};
    use crate::ui::common::TestEnv;
    use crate::system::system;

    #[test]
    fn test_git_stage_ctrl_navigation() {
        let env = TestEnv::setup("test_git_stage_ctrl");
        let _ = system("git init");
        std::fs::write(env.root.join("file1"), "data").unwrap();
        let _ = system("git add file1");
        
        let items = vec![
            GitItem { label: "Header".to_string(), status: None, kind: GitItemKind::Header, path: None },
            GitItem { label: "file1".to_string(), status: Some("s".to_string()), kind: GitItemKind::Entry, path: Some("file1".to_string()) },
            GitItem { label: "file2".to_string(), status: Some("?".to_string()), kind: GitItemKind::Entry, path: Some("file2".to_string()) },
        ];
        
        let mut ctrl = GitStageCtrl::with_items(items).unwrap();
        
        assert_eq!(ctrl.selected_idx, Some(1)); // first selectable is index 1
        assert_eq!(ctrl.focus_file_name(), Some("file1".to_string()));

        ctrl.next().unwrap();
        assert_eq!(ctrl.selected_idx, Some(2));
        assert_eq!(ctrl.focus_file_name(), Some("file2".to_string()));

        ctrl.prev().unwrap();
        assert_eq!(ctrl.selected_idx, Some(1));
    }
}
