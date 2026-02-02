use std::cmp::min;
use std::path::{Path};
use crate::app::AppContext;
use crate::config::RegItem;
use crate::ui::main_ctrl::DirEntry;
use crate::util::{calculate_goto_score, match_disorder};

#[derive(Clone)]
pub enum GotoItem {
    Repo(RegItem),
    LocalDir(DirEntry),
    LocalFile(DirEntry),
}

pub struct GotoCtrl {
    pub items: Vec<GotoItem>,
    pub selected_idx: usize,
    pub filter: String,
}

impl GotoCtrl {
    pub fn new(ctx: &AppContext) -> anyhow::Result<Self> {
        Self::with_repos(ctx.config.path.clone())
    }

    pub fn with_repos(repos: Vec<RegItem>) -> anyhow::Result<Self> {
        let mut items = Vec::new();
        
        for reg in repos {
            items.push(GotoItem::Repo(reg.clone()));
        }
        
        let cwd = std::env::current_dir()?;
        if let Ok(rd) = std::fs::read_dir(&cwd) {
            for entry in rd.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == ".dcdata" || name == ".DS_Store" {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let dir_entry = DirEntry {
                        name: name.to_string(),
                        is_dir,
                    };
                    if is_dir {
                        items.push(GotoItem::LocalDir(dir_entry));
                    } else {
                        items.push(GotoItem::LocalFile(dir_entry));
                    }
                }
            }
        }

        let ctrl = Self {
            items,
            selected_idx: 0,
            filter: String::new(),
        };
        Ok(ctrl)
    }

    pub fn filtered_items(&self) -> Vec<GotoItem> {
        if self.filter.trim().is_empty() {
            return self.items.clone();
        }
        let filter = self.filter.to_lowercase();
        let list: Vec<String> = filter.split_whitespace().map(|s| s.to_string()).collect();
        
        let mut filtered: Vec<GotoItem> = self.items
            .iter()
            .filter(|item| {
                let target = match item {
                    GotoItem::Repo(reg) => reg.path.to_lowercase(),
                    GotoItem::LocalDir(dir) => dir.name.to_lowercase(),
                    GotoItem::LocalFile(file) => file.name.to_lowercase(),
                };
                match_disorder(&target, &list)
            })
            .cloned()
            .collect();

        filtered.sort_by(|a, b| {
            let get_name = |item: &GotoItem| match item {
                GotoItem::Repo(reg) => Path::new(&reg.path).file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase(),
                GotoItem::LocalDir(dir) => dir.name.to_lowercase(),
                GotoItem::LocalFile(file) => file.name.to_lowercase(),
            };
            
            let name_a = get_name(a);
            let name_b = get_name(b);
            
            let score_a = calculate_goto_score(&name_a, &filter, &list);
            let score_b = calculate_goto_score(&name_b, &filter, &list);
            
            if score_a != score_b {
                return score_b.cmp(&score_a);
            }

            let type_score = |item: &GotoItem| match item {
                GotoItem::Repo(_) => 0,
                GotoItem::LocalDir(_) => 1,
                GotoItem::LocalFile(_) => 2,
            };
            
            let ts_a = type_score(a);
            let ts_b = type_score(b);
            if ts_a != ts_b {
                return ts_a.cmp(&ts_b);
            }

            name_a.cmp(&name_b)
        });
        
        filtered
    }

    pub fn focus_item(&self) -> Option<GotoItem> {
        self.filtered_items().get(self.selected_idx).cloned()
    }

    pub fn next(&mut self) {
        let len = self.filtered_items().len();
        self.selected_idx = min(self.selected_idx + 1, len.saturating_sub(1));
    }

    pub fn prev(&mut self) {
        self.selected_idx = self.selected_idx.saturating_sub(1);
    }

    pub fn set_selected(&mut self, idx: usize) {
        if idx < self.filtered_items().len() {
            self.selected_idx = idx;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegItem;

    #[test]
    fn test_goto_ctrl_filtering() {
        let repos = vec![
            RegItem {
                names: vec!["project1".to_string()],
                path: "/path/to/p1".to_string(),
                groups: vec![],
                repo: true,
            },
            RegItem {
                names: vec!["awesome".to_string()],
                path: "/path/to/awesome".to_string(),
                groups: vec![],
                repo: true,
            },
        ];
        
        let mut ctrl = GotoCtrl::with_repos(repos).unwrap();
        
        // Initial items should contain our repos
        assert!(ctrl.items.iter().any(|i| match i {
            GotoItem::Repo(r) => r.names[0] == "project1",
            _ => false,
        }));

        // Test filtering
        ctrl.filter = "awe".to_string();
        let filtered = ctrl.filtered_items();
        assert!(filtered.len() >= 1);
        let has_awesome = filtered.iter().any(|i| match i {
            GotoItem::Repo(r) => r.names[0] == "awesome",
            _ => false,
        });
        assert!(has_awesome);
        
        // Items that don't match should be excluded
        let has_p1 = filtered.iter().any(|i| match i {
            GotoItem::Repo(r) => r.names[0] == "project1",
            _ => false,
        });
        assert!(!has_p1);
    }
}
