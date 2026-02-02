use std::cmp::min;
use std::path::{PathBuf};
use crate::app::AppContext;
use crate::git;
use crate::system::{system_logged};
use crate::util::strip_ansi;

pub struct GitCommitCtrl {
    pub message: String,
    pub files: Vec<String>,
    pub selected_idx: usize,
    pub content: Vec<String>,
    pub commits: Vec<String>,
    pub content_scroll: u16,
    pub input_mode: bool,
    pub repo_root: PathBuf,
}

impl GitCommitCtrl {
    pub fn new(_ctx: &AppContext) -> anyhow::Result<Self> {
        let repo_root = git::repo_root()?;
        let staged = system_logged(
            "GitCommit",
            &git::git_cmd_at(&repo_root, "diff --name-only --staged"),
        )?;
        let mut files = Vec::new();
        for line in staged.lines() {
            if !line.trim().is_empty() {
                files.push(format!("s {}", line));
            }
        }
        if files.is_empty() {
            files.push("< Nothing >".to_string());
        }
        let commits =
            git::commit_list_at(&repo_root).unwrap_or_else(|_| vec!["< There is no commit >".to_string()]);
        Self::with_data(repo_root, files, commits)
    }

    pub fn with_data(repo_root: PathBuf, files: Vec<String>, commits: Vec<String>) -> anyhow::Result<Self> {
        let mut ctrl = Self {
            message: String::new(),
            files,
            selected_idx: 0,
            content: vec!["< Nothing to display >".to_string()],
            commits,
            content_scroll: 0,
            input_mode: true,
            repo_root,
        };
        let _ = ctrl.load_content();
        Ok(ctrl)
    }

    pub fn focus_file_name(&self) -> Option<String> {
        let line = self.files.get(self.selected_idx)?.clone();
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            Some(parts[1].trim().to_string())
        } else {
            None
        }
    }

    pub fn load_content(&mut self) -> anyhow::Result<()> {
        if let Some(name) = self.focus_file_name() {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&self.repo_root)
                .arg("diff")
                .arg("--color")
                .arg("--staged")
                .arg("--")
                .arg(&name)
                .output();
            let (stdout, stderr) = match output {
                Ok(out) => (
                    String::from_utf8_lossy(&out.stdout).to_string(),
                    String::from_utf8_lossy(&out.stderr).to_string(),
                ),
                Err(_) => ("".to_string(), "< no diff >".to_string()),
            };
            let body = if stdout.trim().is_empty() && !stderr.trim().is_empty() {
                stderr
            } else if stdout.trim().is_empty() {
                "< no diff >".to_string()
            } else {
                stdout
            };
            self.content = strip_ansi(&body)
                .replace('\t', "    ")
                .lines()
                .map(|s| s.to_string())
                .collect();
        } else {
            self.content = vec!["< Nothing to display >".to_string()];
        }
        Ok(())
    }

    pub fn next(&mut self) -> anyhow::Result<()> {
        if !self.files.is_empty() {
            self.selected_idx = min(self.selected_idx + 1, self.files.len().saturating_sub(1));
            self.content_scroll = 0;
            self.load_content()?;
        }
        Ok(())
    }

    pub fn prev(&mut self) -> anyhow::Result<()> {
        self.selected_idx = self.selected_idx.saturating_sub(1);
        self.content_scroll = 0;
        self.load_content()?;
        Ok(())
    }

    pub fn set_selected(&mut self, idx: usize) -> anyhow::Result<()> {
        if idx < self.files.len() {
            self.selected_idx = idx;
            self.load_content()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_commit_ctrl_basic() {
        let files = vec!["s file1.rs".to_string(), "s file2.rs".to_string()];
        let commits = vec!["hash1 msg1".to_string()];
        let mut ctrl = GitCommitCtrl::with_data(PathBuf::from("."), files, commits).unwrap();
        
        assert_eq!(ctrl.files.len(), 2);
        assert_eq!(ctrl.focus_file_name(), Some("file1.rs".to_string()));

        ctrl.next().unwrap();
        assert_eq!(ctrl.selected_idx, 1);
        assert_eq!(ctrl.focus_file_name(), Some("file2.rs".to_string()));

        ctrl.prev().unwrap();
        assert_eq!(ctrl.selected_idx, 0);
    }
}
