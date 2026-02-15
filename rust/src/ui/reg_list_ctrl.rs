use std::cmp::min;
use std::collections::HashMap;
use std::sync::{mpsc, Arc};
use std::thread;
use crate::app::AppContext;
use crate::config::RegItem;
use crate::git::{self, PullEvent, PullEventKind, PullStatus, RepoPullInfo, RepoStatusInfo, StatusEvent};
use crate::util::{Semaphore, strip_ansi};

#[derive(PartialEq)]
pub enum DetailMode {
    None,
    Log,
    Status,
}

pub struct RegListCtrl {
    pub items: Vec<RegItem>,
    pub selected_idx: usize,
    pub filter: String,
    pub pull_tx: Option<mpsc::Sender<PullEvent>>,
    pub pull_rx: Option<mpsc::Receiver<PullEvent>>,
    pub pull_sem: Arc<Semaphore>,
    pub pull_infos: HashMap<String, RepoPullInfo>,
    pub pull_total: usize,
    pub pull_done: usize,
    pub status_rx: Option<mpsc::Receiver<StatusEvent>>,
    pub status_infos: HashMap<String, RepoStatusInfo>,
    pub detail_rx: Option<mpsc::Receiver<(String, Vec<String>)>>,
    pub detail_mode: DetailMode,
    pub log_path: Option<String>,
    pub log_scroll: u16,
    pub status_lines: Vec<String>,
}

impl RegListCtrl {
    pub fn new(ctx: &AppContext) -> anyhow::Result<Self> {
        let mut items = ctx.config.path.clone();
        items.sort_by_key(|i| i.path.clone());
        Self::with_repos(items)
    }

    pub fn with_repos(items: Vec<RegItem>) -> anyhow::Result<Self> {
        let mut ctrl = Self {
            items,
            selected_idx: 0,
            filter: String::new(),
            pull_tx: None,
            pull_rx: None,
            pull_sem: Arc::new(Semaphore::new(5)),
            pull_infos: HashMap::new(),
            pull_total: 0,
            pull_done: 0,
            status_rx: None,
            status_infos: HashMap::new(),
            detail_rx: None,
            detail_mode: DetailMode::Status,
            log_path: None,
            log_scroll: u16::default(),
            status_lines: Vec::new(),
        };
        ctrl.start_status_check();
        ctrl.fetch_detail();
        Ok(ctrl)
    }

    pub fn start_status_check(&mut self) {
        let targets: Vec<String> = self.items.iter()
            .filter(|i| i.repo)
            .map(|i| i.path.clone())
            .collect();
        self.start_status_checks_for(targets);
    }

    pub fn start_status_checks_for(&mut self, paths: Vec<String>) {
        if paths.is_empty() { return; }
        
        let (tx, rx) = mpsc::channel();
        self.status_rx = Some(rx);

        let sem = Arc::new(Semaphore::new(10));
        for path in paths {
            let tx = tx.clone();
            let sem = sem.clone();
            thread::spawn(move || {
                sem.acquire();
                git::run_git_stage_check(path, tx);
                sem.release();
            });
        }
    }

    pub fn drain_status_events(&mut self) {
        let Some(rx) = &self.status_rx else { return };
        let mut changed = false;
        loop {
            match rx.try_recv() {
                Ok(ev) => {
                    if let Some(info) = ev.info {
                        self.status_infos.insert(ev.path, info);
                        changed = true;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status_rx = None;
                    break;
                }
            }
        }
        if changed {
            self.sort_items();
        }
    }

    pub fn sort_items(&mut self) {
        let selected_path = self.focus_item().map(|i| i.path.clone());
        self.items.sort_by(|a, b| {
            let a_info = self.status_infos.get(&a.path);
            let b_info = self.status_infos.get(&b.path);
            
            let a_changed = a_info.map(|info| info.dirty || info.ahead > 0 || info.behind > 0).unwrap_or(false);
            let b_changed = b_info.map(|info| info.dirty || info.ahead > 0 || info.behind > 0).unwrap_or(false);
            
            if a_changed != b_changed {
                b_changed.cmp(&a_changed)
            } else {
                a.path.cmp(&b.path)
            }
        });

        if let Some(path) = selected_path {
            let filtered = self.filtered_items();
            if let Some(pos) = filtered.iter().position(|i| i.path == path) {
                self.selected_idx = pos;
            }
        }
    }

    pub fn fetch_detail(&mut self) {
        if let Some(item) = self.focus_item() {
            let path = item.path.clone();
            let (tx, rx) = mpsc::channel();
            self.detail_rx = Some(rx); 
            
            thread::spawn(move || {
                let text = match git::get_git_stage_output(&path) {
                    Ok(out) => out,
                    Err(e) => format!("Error: {}", e),
                };
                
                let lines: Vec<String> = strip_ansi(&text).lines().map(|s| s.to_string()).collect();
                let _ = tx.send((path, lines));
            });
        }
    }

    pub fn drain_detail(&mut self) {
        let Some(rx) = &self.detail_rx else { return };
        match rx.try_recv() {
            Ok((path, lines)) => {
                if self.focus_item().map(|i| i.path) == Some(path) {
                    self.status_lines = lines;
                }
            }
            _ => {}
        }
    }

    pub fn select_at(&mut self, idx: usize) {
        let len = self.filtered_items().len();
        if len == 0 {
            self.selected_idx = 0;
            self.status_lines.clear();
            return;
        }
        
        let new_idx = min(idx, len.saturating_sub(1));
        let old_path = self.focus_item().map(|i| i.path);
        
        self.selected_idx = new_idx;
        let new_path = self.focus_item().map(|i| i.path);
        
        if old_path != new_path {
            self.status_lines.clear(); // Immediately clear panel when path changes
            self.log_path = new_path;
            self.log_scroll = 0;
            self.fetch_detail();
        }
    }

    pub fn filtered_items(&self) -> Vec<RegItem> {
        if self.filter.trim().is_empty() {
            return self.items.clone();
        }
        let filter = self.filter.to_lowercase();
        self.items
            .iter()
            .filter(|i| {
                i.names
                    .iter()
                    .any(|n| n.to_lowercase().contains(&filter))
            })
            .cloned()
            .collect()
    }

    pub fn focus_item(&self) -> Option<RegItem> {
        self.filtered_items().get(self.selected_idx).cloned()
    }

    pub fn next(&mut self) {
        self.select_at(self.selected_idx + 1);
    }

    pub fn prev(&mut self) {
        self.select_at(self.selected_idx.saturating_sub(1));
    }

    pub fn start_pull(&mut self, targets: Vec<RegItem>, is_rebase: bool) {
        if targets.is_empty() {
            return;
        }

        if self.pull_tx.is_none() {
            let (tx, rx) = mpsc::channel();
            self.pull_tx = Some(tx);
            self.pull_rx = Some(rx);
        }
        
        let tx = self.pull_tx.as_ref().unwrap();

        for item in targets {
            if let Some(info) = self.pull_infos.get(&item.path) {
                match info.status {
                    PullStatus::Pending | PullStatus::Running => continue,
                    _ => {}
                }
            }

            self.pull_total += 1;
            self.pull_infos.insert(
                item.path.clone(),
                RepoPullInfo {
                    status: PullStatus::Pending,
                    log: vec![],
                },
            );

            let tx = tx.clone();
            let sem = self.pull_sem.clone();
            let path = item.path.clone();
            thread::spawn(move || {
                sem.acquire();
                let _ = tx.send(PullEvent::started(path.clone()));
                let (code, message) = git::run_git_pull(&path, is_rebase, &tx);
                let _ = tx.send(PullEvent::finished(path.clone(), code, message));
                sem.release();
            });
        }
    }

    pub fn drain_pull_events(&mut self) {
        let Some(rx) = &self.pull_rx else { return };
        let mut finished_paths = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(ev) => match ev.kind {
                    PullEventKind::Started => {
                        if let Some(info) = self.pull_infos.get_mut(&ev.path) {
                            info.status = PullStatus::Running;
                        }
                    }
                    PullEventKind::Line(line) => {
                        if let Some(info) = self.pull_infos.get_mut(&ev.path) {
                            info.log.push(strip_ansi(&line));
                            if info.log.len() > 2000 {
                                let excess = info.log.len() - 2000;
                                info.log.drain(0..excess);
                            }
                        }
                    }
                    PullEventKind::Finished(code, message) => {
                        if let Some(info) = self.pull_infos.get_mut(&ev.path) {
                            info.status = PullStatus::Done { code, message };
                        }
                        self.pull_done = self.pull_done.saturating_add(1);
                        finished_paths.push(ev.path.clone());
                    }
                },
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pull_rx = None;
                    self.pull_tx = None;
                    break;
                }
            }
        }
        
        if !finished_paths.is_empty() {
            self.start_status_checks_for(finished_paths);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegItem;

    #[test]
    fn test_reg_list_ctrl_filtering() {
        let items = vec![
            RegItem { names: vec!["repo1".to_string()], path: "/p1".to_string(), groups: vec![], repo: true },
            RegItem { names: vec!["other".to_string()], path: "/other".to_string(), groups: vec![], repo: true },
        ];
        let mut ctrl = RegListCtrl::with_repos(items).unwrap();
        
        assert_eq!(ctrl.filtered_items().len(), 2);
        
        ctrl.filter = "repo".to_string();
        let filtered = ctrl.filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].names[0], "repo1");

        ctrl.select_at(0);
        assert_eq!(ctrl.focus_item().unwrap().names[0], "repo1");
    }

    #[test]
    fn test_reg_list_ctrl_status_update() {
        let items = vec![
            RegItem { names: vec!["repo1".to_string()], path: "/p1".to_string(), groups: vec![], repo: true },
            RegItem { names: vec!["repo2".to_string()], path: "/p2".to_string(), groups: vec![], repo: true },
        ];
        let mut ctrl = RegListCtrl::with_repos(items).unwrap();
        
        let (tx, rx) = mpsc::channel();
        ctrl.status_rx = Some(rx);
        
        // Mock a status event for repo2 (dirty and ahead)
        tx.send(StatusEvent {
            path: "/p2".to_string(),
            info: Some(RepoStatusInfo {
                branch: "main".to_string(),
                upstream: "origin/main".to_string(),
                dirty: true,
                ahead: 1,
                behind: 0,
            }),
        }).unwrap();
        
        ctrl.drain_status_events();
        
        // Check if status_infos is updated
        assert!(ctrl.status_infos.contains_key("/p2"));
        let info = ctrl.status_infos.get("/p2").unwrap();
        assert!(info.dirty);
        assert_eq!(info.ahead, 1);
        
        // Check if sorted (repo2 should come first because it's changed)
        assert_eq!(ctrl.items[0].path, "/p2");
        assert_eq!(ctrl.items[1].path, "/p1");
    }

    #[test]
    fn test_reg_list_ctrl_clear_detail() {
        let items = vec![
            RegItem { names: vec!["repo1".to_string()], path: "/p1".to_string(), groups: vec![], repo: true },
            RegItem { names: vec!["repo2".to_string()], path: "/p2".to_string(), groups: vec![], repo: true },
        ];
        let mut ctrl = RegListCtrl::with_repos(items).unwrap();
        
        ctrl.status_lines = vec!["some old detail".to_string()];
        
        // Select a different repo
        ctrl.select_at(1);
        
        // Should be cleared immediately
        assert!(ctrl.status_lines.is_empty());
        assert_eq!(ctrl.log_path, Some("/p2".to_string()));
    }

    #[test]
    fn test_reg_list_ctrl_initial_status_check() {
        let items = vec![
            RegItem { names: vec!["repo1".to_string()], path: "/p1".to_string(), groups: vec![], repo: true },
        ];
        let mut ctrl = RegListCtrl::with_repos(items).unwrap();
        
        // Manual call since with_repos skips it for pure logic testing
        ctrl.start_status_check();
        
        // Check if receiver is initialized
        assert!(ctrl.status_rx.is_some());
    }

    #[test]
    fn test_reg_list_ctrl_refresh_after_pull() {
        let items = vec![
            RegItem { names: vec!["repo1".to_string()], path: "/p1".to_string(), groups: vec![], repo: true },
        ];
        let mut ctrl = RegListCtrl::with_repos(items).unwrap();
        
        let (tx, rx) = mpsc::channel();
        ctrl.pull_rx = Some(rx);
        
        // Mock a finished pull event
        tx.send(PullEvent {
            path: "/p1".to_string(),
            kind: PullEventKind::Finished(0, None),
        }).unwrap();
        
        // Before draining, status_rx should be None (or the one from with_repos)
        // Let's force it to None to be sure
        ctrl.status_rx = None;
        
        ctrl.drain_pull_events();
        
        // After draining a Finished event, it should have started a status check
        assert!(ctrl.status_rx.is_some());
    }
}
