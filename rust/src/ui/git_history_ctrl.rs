use std::cmp::min;
use std::path::PathBuf;

use crate::app::AppContext;
use crate::git::{self, CommitSummary};

pub struct GitHistoryCtrl {
    pub repo_root: PathBuf,
    pub commits: Vec<CommitSummary>,
    pub selected_idx: usize,
    pub filter: String,
    pub filtered: Vec<CommitSummary>,
    pub detail: Vec<String>,
    pub detail_scroll: u16,
}

impl GitHistoryCtrl {
    pub fn new(_ctx: &AppContext) -> anyhow::Result<Self> {
        let repo_root = git::repo_root()?;
        let commits = git::commit_history_at(&repo_root, 200)?;
        Self::with_data(repo_root, commits)
    }

    pub fn with_data(repo_root: PathBuf, commits: Vec<CommitSummary>) -> anyhow::Result<Self> {
        let mut ctrl = Self {
            repo_root,
            commits,
            selected_idx: 0,
            filter: String::new(),
            filtered: Vec::new(),
            detail: vec!["< Nothing to display >".to_string()],
            detail_scroll: 0,
        };
        ctrl.apply_filter()?;
        Ok(ctrl)
    }

    pub fn apply_filter(&mut self) -> anyhow::Result<()> {
        if self.filter.trim().is_empty() {
            self.filtered = self.commits.clone();
        } else {
            let f = self.filter.to_lowercase();
            self.filtered = self
                .commits
                .iter()
                .filter(|c| {
                    c.author.to_lowercase().contains(&f) || c.subject.to_lowercase().contains(&f)
                })
                .cloned()
                .collect();
        }
        if self.filtered.is_empty() {
            self.selected_idx = 0;
            self.detail = vec!["< No commit >".to_string()];
            self.detail_scroll = 0;
            return Ok(());
        }
        self.selected_idx = min(self.selected_idx, self.filtered.len().saturating_sub(1));
        self.detail_scroll = 0;
        self.load_detail()?;
        Ok(())
    }

    pub fn focus_commit(&self) -> Option<&CommitSummary> {
        self.filtered.get(self.selected_idx)
    }

    pub fn set_filter(&mut self, filter: String) -> anyhow::Result<()> {
        self.filter = filter;
        self.apply_filter()
    }

    pub fn next(&mut self) -> anyhow::Result<()> {
        if self.filtered.is_empty() {
            return Ok(());
        }
        self.selected_idx = min(self.selected_idx + 1, self.filtered.len().saturating_sub(1));
        self.detail_scroll = 0;
        self.load_detail()?;
        Ok(())
    }

    pub fn prev(&mut self) -> anyhow::Result<()> {
        if self.filtered.is_empty() {
            return Ok(());
        }
        self.selected_idx = self.selected_idx.saturating_sub(1);
        self.detail_scroll = 0;
        self.load_detail()?;
        Ok(())
    }

    pub fn page_down(&mut self) -> anyhow::Result<()> {
        if self.filtered.is_empty() {
            return Ok(());
        }
        self.selected_idx = min(
            self.selected_idx + 10,
            self.filtered.len().saturating_sub(1),
        );
        self.detail_scroll = 0;
        self.load_detail()?;
        Ok(())
    }

    pub fn page_up(&mut self) -> anyhow::Result<()> {
        if self.filtered.is_empty() {
            return Ok(());
        }
        self.selected_idx = self.selected_idx.saturating_sub(10);
        self.detail_scroll = 0;
        self.load_detail()?;
        Ok(())
    }

    pub fn set_selected(&mut self, idx: usize) -> anyhow::Result<()> {
        if idx < self.filtered.len() {
            self.selected_idx = idx;
            self.detail_scroll = 0;
            self.load_detail()?;
        }
        Ok(())
    }

    pub fn load_detail(&mut self) -> anyhow::Result<()> {
        let Some(commit) = self.focus_commit() else {
            self.detail = vec!["< No commit >".to_string()];
            return Ok(());
        };
        match git::commit_detail_at(&self.repo_root, &commit.hash) {
            Ok(lines) if !lines.is_empty() => {
                self.detail = lines;
            }
            Ok(_) => {
                self.detail = vec!["< No detail >".to_string()];
            }
            Err(err) => {
                self.detail = vec![format!("Error loading detail: {}", err)];
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_commits() -> Vec<CommitSummary> {
        vec![
            CommitSummary {
                hash: "aaa1111".to_string(),
                author: "Alice".to_string(),
                date: "2026-02-15".to_string(),
                subject: "Add history screen".to_string(),
            },
            CommitSummary {
                hash: "bbb2222".to_string(),
                author: "Bob".to_string(),
                date: "2026-02-14".to_string(),
                subject: "Fix filter logic".to_string(),
            },
            CommitSummary {
                hash: "ccc3333".to_string(),
                author: "Chris".to_string(),
                date: "2026-02-13".to_string(),
                subject: "Refactor stage screen".to_string(),
            },
        ]
    }

    #[test]
    fn test_git_history_ctrl_filter_by_author_and_subject() {
        let repo = PathBuf::from(".");
        let mut ctrl = GitHistoryCtrl::with_data(repo, sample_commits()).unwrap();

        ctrl.set_filter("alice".to_string()).unwrap();
        assert_eq!(ctrl.filtered.len(), 1);
        assert_eq!(ctrl.filtered[0].author, "Alice");

        ctrl.set_filter("filter".to_string()).unwrap();
        assert_eq!(ctrl.filtered.len(), 1);
        assert_eq!(ctrl.filtered[0].subject, "Fix filter logic");
    }

    #[test]
    fn test_git_history_ctrl_navigation_bounds() {
        let repo = PathBuf::from(".");
        let mut ctrl = GitHistoryCtrl::with_data(repo, sample_commits()).unwrap();

        ctrl.next().unwrap();
        ctrl.next().unwrap();
        ctrl.next().unwrap();
        assert_eq!(ctrl.selected_idx, 2);

        ctrl.prev().unwrap();
        ctrl.prev().unwrap();
        ctrl.prev().unwrap();
        assert_eq!(ctrl.selected_idx, 0);
    }
}
