use regex::Regex;
use std::path::{Path, PathBuf};
use std::io::{Write, BufReader, BufRead};
use std::sync::mpsc;

use crate::system::{system, system_safe, system_logged};
use crate::config::RegItem;
use crate::util::unwrap_quotes_filename;

#[derive(Debug, Clone)]
pub struct BranchStatus {
    pub branch: String,
    pub rev: String,
    pub upstream: String,
    pub remote_rev: String,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Clone, PartialEq)]
pub enum GitItemKind {
    Header,
    Entry,
}

pub struct GitItem {
    pub label: String,
    pub status: Option<String>,
    pub kind: GitItemKind,
    pub path: Option<String>,
}

pub struct RepoStatusInfo {
    pub branch: String,
    pub upstream: String,
    pub dirty: bool,
    pub ahead: usize,
    pub behind: usize,
}

impl RepoStatusInfo {
    pub fn format_status(&self) -> String {
        let mut parts = Vec::new();
        if self.dirty {
            parts.push("*".to_string());
        }
        if self.ahead > 0 {
            parts.push(format!("+{}", self.ahead));
        }
        if self.behind > 0 {
            parts.push(format!("-{}", self.behind));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" {}", parts.join(" "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_status_info_formatting() {
        let clean = RepoStatusInfo {
            branch: "main".to_string(),
            upstream: "origin/main".to_string(),
            dirty: false,
            ahead: 0,
            behind: 0,
        };
        assert_eq!(clean.format_status(), "");

        let dirty = RepoStatusInfo {
            branch: "main".to_string(),
            upstream: "origin/main".to_string(),
            dirty: true,
            ahead: 0,
            behind: 0,
        };
        assert_eq!(dirty.format_status(), " *");

        let ahead = RepoStatusInfo {
            branch: "main".to_string(),
            upstream: "origin/main".to_string(),
            dirty: false,
            ahead: 5,
            behind: 0,
        };
        assert_eq!(ahead.format_status(), " +5");

        let behind = RepoStatusInfo {
            branch: "main".to_string(),
            upstream: "origin/main".to_string(),
            dirty: false,
            ahead: 0,
            behind: 3,
        };
        assert_eq!(behind.format_status(), " -3");

        let complex = RepoStatusInfo {
            branch: "main".to_string(),
            upstream: "origin/main".to_string(),
            dirty: true,
            ahead: 2,
            behind: 1,
        };
        assert_eq!(complex.format_status(), " * +2 -1");
    }
}

pub struct StatusEvent {
    pub path: String,
    pub info: Option<RepoStatusInfo>,
}

pub struct RepoPullInfo {
    pub status: PullStatus,
    pub log: Vec<String>,
}

pub enum PullStatus {
    Pending,
    Running,
    Done { code: i32, message: Option<String> },
}

impl PullStatus {
    pub fn label(&self) -> &'static str {
        match self {
            PullStatus::Pending => "PEND",
            PullStatus::Running => "RUN",
            PullStatus::Done { code, .. } => {
                if *code == 0 {
                    "OK"
                } else {
                    "ERR"
                }
            }
        }
    }
}

pub struct PullEvent {
    pub path: String,
    pub kind: PullEventKind,
}

pub enum PullEventKind {
    Started,
    Line(String),
    Finished(i32, Option<String>),
}

impl PullEvent {
    pub fn started(path: String) -> Self {
        Self {
            path,
            kind: PullEventKind::Started,
        }
    }

    pub fn finished(path: String, code: i32, message: Option<String>) -> Self {
        Self {
            path,
            kind: PullEventKind::Finished(code, message),
        }
    }
}

#[derive(Clone, Copy)]
pub enum GitAction {
    Fetch,
    Merge,
    Status,
    Update,
}

pub struct GitActor {
    pub is_pull_rebase: bool,
    pub repo_list: Vec<RegItem>,
}

impl GitActor {
    pub fn new(is_pull_rebase: bool, repo_list: Vec<RegItem>) -> Self {
        Self {
            is_pull_rebase,
            repo_list,
        }
    }

    pub fn action(&mut self, action: GitAction, target: Option<&str>) -> anyhow::Result<bool> {
        if let Some(target) = target {
            return self.apply(action, target);
        }
        for repo in self.repo_list.clone() {
            if let Some(name) = repo.names.get(0) {
                if !self.apply(action, name)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn apply(&mut self, action: GitAction, target: &str) -> anyhow::Result<bool> {
        match action {
            GitAction::Fetch => self.act_fetch(target),
            GitAction::Merge => self.act_merge(target),
            GitAction::Status => self.act_status(target),
            GitAction::Update => self.act_update(target),
        }
    }

    fn act_fetch(&mut self, name: &str) -> anyhow::Result<bool> {
        let path = self.change_path(name)?;
        println!("fetch --prune - {}", path.to_string_lossy());
        let _ = system("LANG=C git fetch --prune");
        Ok(true)
    }

    fn act_merge(&mut self, name: &str) -> anyhow::Result<bool> {
        let path = self.change_path(name)?;
        let branch = get_current_branch()?;
        let remote = match get_tracking_branch() {
            Some(r) => r,
            None => {
                println!("{} DONT'T HAVE TRACKING branch", branch);
                return Ok(false);
            }
        };
        let same = self.check_same_with(name, &branch, &remote)?;
        if same {
            return Ok(true);
        }
        let diff = check_rebaseable(&branch, &remote)?;
        if !diff.is_empty() {
            println!("NOT be able to fast forward - {}", path.to_string_lossy());
        } else {
            println!("merge with {} - {}", remote, path.to_string_lossy());
            let _ = system(&format!("LANG=C git rebase {}", remote));
        }
        Ok(true)
    }

    fn act_status(&mut self, name: &str) -> anyhow::Result<bool> {
        let _path = self.change_path(name)?;
        if !self.stash_check(name)? {
            return Ok(false);
        }
        let branch = get_current_branch()?;
        let remote = match get_tracking_branch() {
            Some(r) => r,
            None => {
                println!("{} DONT'T HAVE TRACKING branch", branch);
                return Ok(false);
            }
        };
        let same = self.check_same_with(name, &branch, &remote)?;
        if same {
            if let Ok(out) = system("LANG=C git status -s") {
                if !out.is_empty() {
                    println!("{out}");
                }
            }
        } else {
            let diff = check_rebaseable(&branch, &remote)?;
            if diff.is_empty() {
                println!("Be able to fast-forward...");
            } else {
                println!("NOT be able to fast forward");
            }
        }
        Ok(true)
    }

    fn act_update(&mut self, name: &str) -> anyhow::Result<bool> {
        let _ = self.act_pull(name)?;
        let _ = self.act_status(name)?;
        Ok(true)
    }

    fn act_pull(&mut self, name: &str) -> anyhow::Result<bool> {
        let path = self.change_path(name)?;
        let mut cmd = "pull".to_string();
        if self.is_pull_rebase {
            cmd.push_str(" -r");
        }
        println!("{} - {}", cmd, path.to_string_lossy());
        let (_out, code) = system_safe(&format!("LANG=C git {}", cmd));
        Ok(code == 0)
    }

    fn stash_check(&mut self, _name: &str) -> anyhow::Result<bool> {
        let stash = stash_get_name_safe("###groupRepo###")?;
        if stash.is_some() {
            println!("YOU HAVE STASH ITEM. PROCESS IT FIRST");
            return Ok(false);
        }
        Ok(true)
    }

    fn check_same_with(
        &mut self,
        name: &str,
        branch: &str,
        remote: &str,
    ) -> anyhow::Result<bool> {
        let current_rev = rev(branch)?;
        let rev2 = rev(&format!("remotes/{}", remote))?;
        if current_rev == rev2 {
            println!("{} -> {} is same to {}", name, branch, remote);
            return Ok(true);
        }
        let common = common_parent_rev(branch, remote)?;
        if common != rev2 {
            println!("{} -> Different", name);
            return Ok(false);
        }
        let gap = commit_gap(branch, remote)?;
        println!(
            "Your local branch({}) is forward than {}[{} commits]",
            branch, remote, gap
        );
        println!("{}", commit_log_between(branch, remote)?);
        Ok(true)
    }

    fn change_path(&mut self, name: &str) -> anyhow::Result<PathBuf> {
        let path = if name.starts_with('/') {
            PathBuf::from(name)
        } else {
            let repo = self
                .repo_list
                .iter()
                .find(|r| r.names.contains(&name.to_string()))
                .ok_or_else(|| anyhow::anyhow!("Can't find repo[name:{}]", name))?;
            PathBuf::from(&repo.path)
        };
        if !path.is_dir() {
            return Err(anyhow::anyhow!("{} doesn't exist", path.to_string_lossy()));
        }
        std::env::set_current_dir(&path)?;
        Ok(path)
    }
}

pub fn git_file_last_name(line: &str) -> Option<String> {
    let text = line.trim();
    let first_space = text.find(' ')?;
    let rest = text[first_space + 1..].trim();
    if let Some(pos) = rest.rfind(" -> ") {
        let target = &rest[pos + 4..];
        return Some(unwrap_quotes_filename(target));
    }
    Some(unwrap_quotes_filename(rest))
}

pub fn git_cmd_at(root: &Path, cmd: &str) -> String {
    format!("git -C \"{}\" {}", root.to_string_lossy(), cmd)
}

pub fn build_git_items() -> anyhow::Result<Vec<GitItem>> {
    let list = status_file_list()?;
    let mut modified = Vec::new();
    let mut untracked = Vec::new();
    let mut staged = Vec::new();
    for (line, status_code) in list {
        if status_code == "??" {
            untracked.push((line, "?".to_string()));
            continue;
        }
        let bytes = status_code.as_bytes();
        let staged_flag = bytes.get(0).copied().unwrap_or(b' ') != b' ';
        let modified_flag = bytes.get(1).copied().unwrap_or(b' ') != b' ';
        if staged_flag {
            staged.push((line.clone(), "s".to_string()));
        }
        if modified_flag {
            modified.push((line, "".to_string()));
        }
    }
    let mut items = Vec::new();
    if !modified.is_empty() {
        items.push(GitItem {
            label: "< Modified >".to_string(),
            status: None,
            kind: GitItemKind::Header,
            path: None,
        });
        for (clean, status) in modified {
            items.push(GitItem {
                label: clean.clone(),
                status: Some(status),
                kind: GitItemKind::Entry,
                path: git_file_last_name(&clean),
            });
        }
    }
    if !untracked.is_empty() {
        items.push(GitItem {
            label: "< Untracked >".to_string(),
            status: None,
            kind: GitItemKind::Header,
            path: None,
        });
        for (clean, status) in untracked {
            items.push(GitItem {
                label: clean.clone(),
                status: Some(status),
                kind: GitItemKind::Entry,
                path: git_file_last_name(&clean),
            });
        }
    }
    if !staged.is_empty() {
        items.push(GitItem {
            label: "< Staged >".to_string(),
            status: None,
            kind: GitItemKind::Header,
            path: None,
        });
        for (clean, status) in staged {
            items.push(GitItem {
                label: clean.clone(),
                status: Some(status),
                kind: GitItemKind::Entry,
                path: git_file_last_name(&clean),
            });
        }
    }
    Ok(items)
}

pub fn run_git_stage_check(path: String, tx: mpsc::Sender<StatusEvent>) {
    let output_branch = std::process::Command::new("git")
        .args(&["-c", "color.ui=false", "rev-parse", "--abbrev-ref", "HEAD"])
        .env("LANG", "C")
        .current_dir(&path)
        .output();

    let branch = match output_branch {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => return, // Not a repo or git missing
    };

    let output_upstream = std::process::Command::new("git")
        .args(&["-c", "color.ui=false", "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .env("LANG", "C")
        .current_dir(&path)
        .output();

    let upstream = match output_upstream {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => String::new(),
    };

    let output_status = std::process::Command::new("git")
        .args(&["-c", "color.status=false", "status", "--porcelain"])
        .env("LANG", "C")
        .current_dir(&path)
        .output();

    let dirty = match output_status {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => false,
    };

    let mut ahead = 0;
    let mut behind = 0;
    if !upstream.is_empty() {
        let output_counts = std::process::Command::new("git")
            .args(&["-c", "color.ui=false", "rev-list", "--count", "--left-right", "HEAD...@{u}"])
            .env("LANG", "C")
            .current_dir(&path)
            .output();
        if let Ok(out) = output_counts {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() == 2 {
                ahead = parts[0].parse().unwrap_or(0);
                behind = parts[1].parse().unwrap_or(0);
            }
        }
    }

    let _ = tx.send(StatusEvent {
        path,
        info: Some(RepoStatusInfo {
            branch,
            upstream,
            dirty,
            ahead,
            behind,
        }),
    });
}

pub fn run_git_pull(path: &str, is_rebase: bool, tx: &mpsc::Sender<PullEvent>) -> (i32, Option<String>) {
    let cmd = if is_rebase {
        "LANG=C git fetch -p && LANG=C git pull -r 2>&1"
    } else {
        "LANG=C git fetch -p && LANG=C git pull 2>&1"
    };
    let mut child = match std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(path)
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            let _ = tx.send(PullEvent {
                path: path.to_string(),
                kind: PullEventKind::Line(format!("spawn error: {}", err)),
            });
            return (1, Some(err.to_string()));
        }
    };
    let mut last_line: Option<String> = None;
    let mut err_line: Option<String> = None;
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            let line_trim = line.trim().to_string();
            if !line_trim.is_empty() {
                last_line = Some(line_trim.clone());
                if line_trim.to_lowercase().starts_with("error") {
                    err_line = Some(line_trim.clone());
                }
            }
            let _ = tx.send(PullEvent {
                path: path.to_string(),
                kind: PullEventKind::Line(line),
            });
        }
    }
    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    let message = if code == 0 { None } else { err_line.or(last_line) };
    (code, message)
}

pub fn run_action(is_pull_rebase: bool, repo_list: Vec<RegItem>, action: GitAction, target: Option<&str>) -> anyhow::Result<()> {
    let mut actor = GitActor::new(is_pull_rebase, repo_list);
    actor.action(action, target)?;
    Ok(())
}

pub fn get_git_stage_output(path: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .arg("-c").arg("color.status=never")
        .arg("status")
        .env("LANG", "C")
        .current_dir(path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn rev(branch: &str) -> anyhow::Result<String> {
    let output = system("LANG=C git branch -va")?;
    let re = Regex::new(&format!(r"^[*]?\s+{}\s+(\w+)", regex::escape(branch)))?;
    let caps = re
        .captures_iter(&output)
        .next()
        .ok_or_else(|| anyhow::anyhow!("branch not found: {}", branch))?;
    Ok(caps[1].to_string())
}

pub fn get_current_branch() -> anyhow::Result<String> {
    system("LANG=C git rev-parse --abbrev-ref HEAD").map_err(Into::into)
}

pub fn repo_root() -> anyhow::Result<PathBuf> {
    let out = system("LANG=C git rev-parse --show-toplevel")?;
    Ok(PathBuf::from(out.trim()))
}

pub fn remote_list() -> anyhow::Result<Vec<String>> {
    let out = system("LANG=C git remote")?;
    Ok(out.lines().map(|s| s.trim().to_string()).collect())
}

pub fn get_tracking_branch() -> Option<String> {
    system("LANG=C git rev-parse --abbrev-ref --symbolic-full-name @{u}").ok()
}

pub fn common_parent_rev(br1: &str, br2: &str) -> anyhow::Result<String> {
    system(&format!("LANG=C git merge-base {} {}", br1, br2)).map_err(Into::into)
}

pub fn print_status() -> anyhow::Result<()> {
    let out = system("LANG=C git -c color.status=always status -s")?;
    println!("{out}\n");
    Ok(())
}

pub fn commit_gap(new_branch: &str, old_branch: &str) -> anyhow::Result<usize> {
    let out = system(&format!("LANG=C git rev-list --count {}..{}", old_branch, new_branch))?;
    Ok(out.trim().parse::<usize>()?)
}

pub fn commit_log_between(new_branch: &str, old_branch: &str) -> anyhow::Result<String> {
    let cmd = format!(
        "LANG=C git log --color --oneline --graph --decorate --abbrev-commit {}^..{}",
        old_branch, new_branch
    );
    match system(&cmd) {
        Ok(out) => Ok(out),
        Err(_) => system(&format!(
            "LANG=C git log --color --oneline --graph --decorate --abbrev-commit {}..{}",
            old_branch, new_branch
        ))
        .map_err(Into::into),
    }
}

pub fn get_branch_status() -> anyhow::Result<Option<BranchStatus>> {
    let out = system("LANG=C git -c color.branch=false branch -avv")?;
    let re = Regex::new(r"^\*\s(\S+)\s+(\w+)\s(.+)")?;
    let caps = match re.captures_iter(&out).next() {
        Some(c) => c,
        None => return Ok(None),
    };
    let branch = caps[1].to_string();
    let rev = caps[2].to_string();
    let line = caps[3].to_string();

    let mut upstream = String::new();
    let mut ahead = 0;
    let mut behind = 0;
    if let Some(info) = Regex::new(r"^\[(.+?)\]")?.captures(&line) {
        let infos = info[1].split(':').collect::<Vec<_>>();
        upstream = infos[0].to_string();
        if infos.len() > 1 {
            for part in infos[1].split(',') {
                let bits = part.trim().split_whitespace().collect::<Vec<_>>();
                if bits.len() == 2 {
                    if bits[0] == "ahead" {
                        ahead = bits[1].parse::<usize>()?;
                    } else if bits[0] == "behind" {
                        behind = bits[1].parse::<usize>()?;
                    }
                }
            }
        }
    }

    let mut remote_rev = String::new();
    if !upstream.is_empty() {
        let re2 = Regex::new(&format!(r"\s\sremotes/{}\s+(\w+)", regex::escape(&upstream)))?;
        if let Some(caps2) = re2.captures(&out) {
            remote_rev = caps2[1].to_string();
        }
    }

    Ok(Some(BranchStatus {
        branch,
        rev,
        upstream,
        remote_rev,
        ahead,
        behind,
    }))
}

pub fn check_rebaseable(br1: &str, br2: &str) -> anyhow::Result<Vec<String>> {
    let common = common_parent_rev(br1, br2)?;
    let br1_diff = system(&format!("LANG=C git diff --name-only {} {}", common, br1))?;
    let br2_diff = system(&format!("LANG=C git diff --name-only {} {}", common, br2))?;
    let list1: Vec<&str> = br1_diff.split_whitespace().collect();
    let list2: Vec<&str> = br2_diff.split_whitespace().collect();
    let mut overlap = Vec::new();
    for s in list1 {
        if list2.contains(&s) {
            overlap.push(s.to_string());
        }
    }
    Ok(overlap)
}

pub fn fetch() -> (String, i32) {
    system_safe("LANG=C git fetch --prune")
}

pub fn rebase(branch: &str) -> (String, i32) {
    system_safe(&format!("LANG=C git rebase {}", branch))
}

pub fn rebase_abort() -> anyhow::Result<String> {
    system("LANG=C git rebase --abort").map_err(Into::into)
}

pub fn stash_get_name_safe(name: &str) -> anyhow::Result<Option<String>> {
    let out = system("LANG=C git stash list")?;
    let re = Regex::new(&format!(r"^(stash@\{{\d+\}}):\s.+: {}", regex::escape(name)))?;
    if let Some(caps) = re.captures(&out) {
        Ok(Some(caps[1].to_string()))
    } else {
        Ok(None)
    }
}


pub fn commit_list_at(root: &Path) -> anyhow::Result<Vec<String>> {
    let out = system(&format!(
        r#"LANG=C git -C "{}" -c color.status=always log --pretty=format:"%h %Cblue%an%Creset(%ar): %Cgreen%s" --graph -4"#,
        root.to_string_lossy()
    ))?;
    Ok(out.lines().map(|l| l.to_string()).collect())
}

pub fn commit_list() -> anyhow::Result<Vec<String>> {
    let root = repo_root()?;
    commit_list_at(&root)
}

pub fn status_file_list() -> anyhow::Result<Vec<(String, String)>> {
    // Get status without color for reliable parsing
    let out = system_logged("GitStage", "LANG=C git status -s")?;
    let mut list = Vec::new();
    for line in out.lines() {
        if line.len() < 3 {
            continue;
        }
        let status_code = line[0..2].to_string();
        list.push((line.to_string(), status_code));
    }
    Ok(list)
}

pub fn add_to_gitignore(path: &str) -> anyhow::Result<()> {
    let root = repo_root()?;
    let gitignore_path = root.join(".gitignore");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(gitignore_path)?;
    writeln!(file, "{}", path)?;
    Ok(())
}

