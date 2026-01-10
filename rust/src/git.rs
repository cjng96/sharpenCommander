use regex::Regex;
use std::path::{Path, PathBuf};

use crate::system::{system, system_safe, system_logged};

#[derive(Debug, Clone)]
pub struct BranchStatus {
    pub branch: String,
    pub rev: String,
    pub upstream: String,
    pub remote_rev: String,
    pub ahead: usize,
    pub behind: usize,
}

pub fn rev(branch: &str) -> anyhow::Result<String> {
    let output = system("git branch -va")?;
    let re = Regex::new(&format!(r"^[*]?\s+{}\s+(\w+)", regex::escape(branch)))?;
    let caps = re
        .captures_iter(&output)
        .next()
        .ok_or_else(|| anyhow::anyhow!("branch not found: {}", branch))?;
    Ok(caps[1].to_string())
}

pub fn get_current_branch() -> anyhow::Result<String> {
    system("git rev-parse --abbrev-ref HEAD").map_err(Into::into)
}

pub fn repo_root() -> anyhow::Result<PathBuf> {
    let out = system("git rev-parse --show-toplevel")?;
    Ok(PathBuf::from(out.trim()))
}

pub fn remote_list() -> anyhow::Result<Vec<String>> {
    let out = system("git remote")?;
    Ok(out.lines().map(|s| s.trim().to_string()).collect())
}

pub fn get_tracking_branch() -> Option<String> {
    system("git rev-parse --abbrev-ref --symbolic-full-name @{u}").ok()
}

pub fn common_parent_rev(br1: &str, br2: &str) -> anyhow::Result<String> {
    system(&format!("git merge-base {} {}", br1, br2)).map_err(Into::into)
}

pub fn print_status() -> anyhow::Result<()> {
    let out = system("git -c color.status=always status -s")?;
    println!("{out}\n");
    Ok(())
}

pub fn commit_gap(new_branch: &str, old_branch: &str) -> anyhow::Result<usize> {
    let out = system(&format!("git rev-list --count {}..{}", old_branch, new_branch))?;
    Ok(out.trim().parse::<usize>()?)
}

pub fn commit_log_between(new_branch: &str, old_branch: &str) -> anyhow::Result<String> {
    let cmd = format!(
        "git log --color --oneline --graph --decorate --abbrev-commit {}^..{}",
        old_branch, new_branch
    );
    match system(&cmd) {
        Ok(out) => Ok(out),
        Err(_) => system(&format!(
            "git log --color --oneline --graph --decorate --abbrev-commit {}..{}",
            old_branch, new_branch
        ))
        .map_err(Into::into),
    }
}

pub fn get_branch_status() -> anyhow::Result<Option<BranchStatus>> {
    let out = system("LANG=en_US git -c color.branch=false branch -avv")?;
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
    let br1_diff = system(&format!("git diff --name-only {} {}", common, br1))?;
    let br2_diff = system(&format!("git diff --name-only {} {}", common, br2))?;
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
    system_safe("git fetch --prune")
}

pub fn rebase(branch: &str) -> (String, i32) {
    system_safe(&format!("git rebase {}", branch))
}

pub fn rebase_abort() -> anyhow::Result<String> {
    system("git rebase --abort").map_err(Into::into)
}

pub fn stash_get_name_safe(name: &str) -> anyhow::Result<Option<String>> {
    let out = system("git stash list")?;
    let re = Regex::new(&format!(r"^(stash@\{{\d+\}}):\s.+: {}", regex::escape(name)))?;
    if let Some(caps) = re.captures(&out) {
        Ok(Some(caps[1].to_string()))
    } else {
        Ok(None)
    }
}


pub fn commit_list_at(root: &Path) -> anyhow::Result<Vec<String>> {
    let out = system(&format!(
        r#"git -C "{}" -c color.status=always log --pretty=format:"%h %Cblue%an%Creset(%ar): %Cgreen%s" --graph -4"#,
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
    let out = system_logged("GitStatus", "git status -s")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_branch_status_none() {
        let data = "* master 1234567 [origin/master] msg";
        let re = Regex::new(r"^\*\s(\S+)\s+(\w+)\s(.+)").unwrap();
        assert!(re.is_match(data));
    }
}
