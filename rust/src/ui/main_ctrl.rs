use std::cmp::min;
use std::path::{PathBuf};
use crate::app::AppContext;

#[derive(Clone, Debug, PartialEq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

pub struct MainCtrl {
    pub cwd: PathBuf,
    pub items: Vec<DirEntry>,
    pub selected_idx: usize,
    pub input: String,
    pub input_mode: bool,
    pub cmd_list: Vec<String>,
    pub work_list: Vec<PathBuf>,
    pub work_idx: usize,
    pub registered_paths: Vec<String>,
    pub confirm_delete: bool,
    pub confirm_target: Option<String>,
}

impl MainCtrl {
    pub fn new(ctx: &AppContext) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let registered_paths = ctx.config.path.iter().map(|i| i.path.clone()).collect();
        Self::with_ctx(cwd, registered_paths)
    }

    pub fn with_ctx(cwd: PathBuf, registered_paths: Vec<String>) -> anyhow::Result<Self> {
        let mut ctrl = Self {
            cwd,
            items: Vec::new(),
            selected_idx: 0,
            input: String::new(),
            input_mode: false,
            cmd_list: Vec::new(),
            work_list: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            work_idx: 0,
            registered_paths,
            confirm_delete: false,
            confirm_target: None,
        };
        ctrl.refresh();
        Ok(ctrl)
    }

    pub fn refresh(&mut self) {
        let mut list = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.cwd) {
            for entry in rd.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == ".dcdata" {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    list.push(DirEntry {
                        name: name.to_string(),
                        is_dir,
                    });
                }
            }
        }
        list.sort_by_key(|e| e.name.clone());
        self.items = list;
        self.items.insert(
            0,
            DirEntry {
                name: "..".to_string(),
                is_dir: true,
            },
        );

        self.selected_idx = 0;
    }

    pub fn focus_name(&self) -> Option<String> {
        self.items.get(self.selected_idx).map(|e| e.name.clone())
    }

    pub fn enter_dir(&mut self, name: &str) -> bool {
        if name == ".." {
            if let Some(parent) = self.cwd.parent() {
                if std::env::set_current_dir(parent).is_ok() {
                    self.cwd = parent.to_path_buf();
                    self.refresh();
                    return true;
                }
            }
            return false;
        }
        let target = self.cwd.join(name);
        if target.is_dir() {
            if std::env::set_current_dir(&target).is_ok() {
                self.cwd = target;
                self.refresh();
                return true;
            }
        }
        false
    }

    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected_idx = min(self.selected_idx + 1, self.items.len().saturating_sub(1));
        }
    }

    pub fn select_prev(&mut self) {
        self.selected_idx = self.selected_idx.saturating_sub(1);
    }

    pub fn set_selected(&mut self, idx: usize) {
        if idx < self.items.len() {
            self.selected_idx = idx;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::ui::common::TestEnv;

    #[test]
    fn test_main_ctrl_navigation() {
        let env = TestEnv::setup("test_main_ctrl");
        
        // Create dummy files and dirs
        fs::create_dir(env.root.join("dir1")).unwrap();
        fs::write(env.root.join("file1.txt"), "hello").unwrap();
        
        let mut ctrl = MainCtrl::with_ctx(env.root.clone(), vec![]).unwrap();
        
        // Test refresh and initial state
        assert_eq!(ctrl.items[0].name, "..");
        assert!(ctrl.items.iter().any(|i| i.name == "dir1" && i.is_dir));
        assert!(ctrl.items.iter().any(|i| i.name == "file1.txt" && !i.is_dir));
        
        // Test focus_name
        ctrl.set_selected(0);
        assert_eq!(ctrl.focus_name(), Some("..".to_string()));
        
        // Test selection logic
        ctrl.select_next();
        assert_eq!(ctrl.selected_idx, 1);
        ctrl.select_prev();
        assert_eq!(ctrl.selected_idx, 0);
        
        // Test enter_dir (to subdir)
        let dir1_idx = ctrl.items.iter().position(|i| i.name == "dir1").unwrap();
        ctrl.set_selected(dir1_idx);
        let name = ctrl.focus_name().unwrap();
        assert!(ctrl.enter_dir(&name));
        assert!(ctrl.cwd.ends_with("dir1"));
        
        // Test enter_dir (to parent)
        assert!(ctrl.enter_dir(".."));
        assert_eq!(ctrl.cwd, env.root);
    }

    #[test]
    fn test_main_ctrl_selection_bounds() {
        let env = TestEnv::setup("test_main_ctrl_bounds");
        let mut ctrl = MainCtrl::with_ctx(env.root.clone(), vec![]).unwrap();
        
        // 0 is ".."
        ctrl.set_selected(0);
        ctrl.select_prev();
        assert_eq!(ctrl.selected_idx, 0);
        
        let last_idx = ctrl.items.len() - 1;
        ctrl.set_selected(last_idx);
        ctrl.select_next();
        assert_eq!(ctrl.selected_idx, last_idx);
    }

    #[test]
    fn test_main_ctrl_refresh_filtering() {
        let env = TestEnv::setup("test_main_ctrl_filter");
        fs::create_dir(env.root.join(".dcdata")).unwrap(); // Should be filtered out
        fs::create_dir(env.root.join("visible_dir")).unwrap();
        
        let ctrl = MainCtrl::with_ctx(env.root.clone(), vec![]).unwrap();
        assert!(!ctrl.items.iter().any(|i| i.name == ".dcdata"));
        assert!(ctrl.items.iter().any(|i| i.name == "visible_dir"));
    }
}
