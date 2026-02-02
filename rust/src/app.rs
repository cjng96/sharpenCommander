use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, RegItem};
use crate::git;
use crate::system::{app_log, system_safe, system_ret};
use crate::ui;

pub struct AppContext {
    pub config: Config,
    pub config_path: PathBuf,
}

impl AppContext {
    pub fn load() -> anyhow::Result<Self> {
        let (config, config_path) = Config::load_or_create()?;
        Ok(Self { config, config_path })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.config.save(&self.config_path)
    }

    pub fn reg_find_by_name(&self, target: &str) -> anyhow::Result<RegItem> {
        for item in &self.config.path {
            if item
                .names
                .iter()
                .any(|n| n.to_lowercase() == target.to_lowercase())
            {
                return Ok(item.clone());
            }
        }
        Err(anyhow::anyhow!("No that target[{}]", target))
    }

    pub fn reg_find_by_path(&self, pp: &str) -> Option<RegItem> {
        self.config.path.iter().find(|x| x.path == pp).cloned()
    }

    pub fn reg_find_items(&self, sub: &str) -> Vec<RegItem> {
        let sub = sub.to_lowercase();
        self.config
            .path
            .iter()
            .filter(|x| {
                x.names
                    .iter()
                    .any(|n| n.to_lowercase().contains(&sub))
            })
            .cloned()
            .collect()
    }

    pub fn reg_add(&mut self, pp: &str) -> anyhow::Result<()> {
        let old = env::current_dir()?;
        env::set_current_dir(pp)?;
        let (out, code) = system_safe("git rev-parse --is-inside-work-tree");
        env::set_current_dir(old)?;
        let is_repo = code == 0 && out.trim() == "true";
        let name = Path::new(pp)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(pp)
            .to_string();
        self.config.path.push(RegItem {
            names: vec![name],
            path: pp.to_string(),
            groups: vec![],
            repo: is_repo,
        });
        self.save()
    }

    pub fn reg_remove(&mut self, pp: &str) -> anyhow::Result<bool> {
        if let Some(pos) = self.config.path.iter().position(|x| x.path == pp) {
            self.config.path.remove(pos);
            self.save()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn save_path(&self, pp: &str) -> anyhow::Result<()> {
        let expanded = crate::system::expand_tilde(pp);
        fs::write("/tmp/cmdDevTool.path", expanded.to_string_lossy().as_bytes())?;
        Ok(())
    }
}

fn setup_sc() -> anyhow::Result<()> {
    if env::var("SC_OK").is_ok() {
        return Ok(());
    }
    println!("SC_OK is not set. Please source script-bash.sh and re-run.");
    Err(anyhow::anyhow!("SC_OK is not set"))
}

pub fn run() -> anyhow::Result<()> {
    app_log("App start");
    setup_sc()?;
    let _ = fs::remove_file("/tmp/cmdDevTool.path");

    let mut args: Vec<String> = env::args().collect();
    let mut initial_path = None;
    let mut i = 0;
    while i < args.len() {
        if args[i].starts_with("--path=") {
            initial_path = Some(args[i][7..].to_string());
            args.remove(i);
        } else {
            i += 1;
        }
    }

    if let Some(p) = initial_path {
        let p = crate::system::expand_tilde(&p);
        env::set_current_dir(&p)?;
    }

    let mut ctx = AppContext::load()?;
    let cur = env::current_dir()?;
    ctx.save_path(cur.to_string_lossy().as_ref())?;

    let res = (|| -> anyhow::Result<()> {
        if args.len() <= 1 {
            // If it is UI mode, it already uses env::current_dir() in MainState::new
            return ui::run(&mut ctx);
        }
        let cmd = &args[1];
        let mut target = if args.len() >= 3 {
            Some(args[2].clone())
        } else {
            None
        };
        if let Some(t) = &target {
            if t == "." {
                if let Ok(cur) = env::current_dir() {
                    target = Some(cur.to_string_lossy().to_string());
                }
            }
        }

        match cmd.as_str() {
            "push" => {
                println!("Fetching first...");
                git::fetch();
                ui::git_push(&mut ctx)?;
            }
            "ci" => {
                ui::run_git_status(&mut ctx)?;
            }
            "list" => {
                for item in &ctx.config.path {
                    println!("{:?}", item);
                }
            }
            "config" => {
                let p = crate::system::expand_tilde("~/.synapcmd");
                if p.is_dir() {
                    let _ = env::set_current_dir(&p);
                } else if let Some(parent) = p.parent() {
                    let _ = env::set_current_dir(parent);
                }
            }
            "which" => {
                let cmdline = args[1..].join(" ");
                let (out, _code) = system_safe(&cmdline);
                println!("{out}");
                if let Some(pp) = Path::new(&out).parent() {
                    let _ = env::set_current_dir(pp);
                }
            }
            "find" => {
                ui::run_find(&mut ctx, &args[1..])?;
            }
            "grep" => {
                ui::run_grep(&mut ctx, &args[1..])?;
            }
            "st" => {
                git::run_action(ctx.config.is_pull_rebase, ctx.config.path.clone(), git::GitAction::Status, target.as_deref())?;
            }
            "fetch" => {
                git::run_action(ctx.config.is_pull_rebase, ctx.config.path.clone(), git::GitAction::Fetch, target.as_deref())?;
            }
            "merge" => {
                git::run_action(ctx.config.is_pull_rebase, ctx.config.path.clone(), git::GitAction::Merge, target.as_deref())?;
            }
            "update" => {
                git::run_action(ctx.config.is_pull_rebase, ctx.config.path.clone(), git::GitAction::Update, target.as_deref())?;
            }
            _ => {
                if cmd == "." {
                    // Already in current dir
                } else {
                    let item = ctx.reg_find_by_name(cmd)?;
                    let _ = env::set_current_dir(&item.path);
                }
            }
        }
        Ok(())
    })();

    if let Ok(cur) = env::current_dir() {
        let _ = ctx.save_path(cur.to_string_lossy().as_ref());
    }
    res
}

pub fn open_in_editor(edit_app: &str, target: &str) {
    let _ = system_ret(&format!("{} {}", edit_app, target));
}

