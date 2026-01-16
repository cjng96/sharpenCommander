use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::system::{expand_tilde, is_executable_in_path};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum NamesField {
    Single(String),
    Many(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegItem {
    #[serde(default)]
    pub names: Vec<String>,
    pub path: String,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub repo: bool,
}

impl RegItem {
    pub fn normalized(mut self) -> Self {
        self.path = expand_tilde(&self.path).to_string_lossy().to_string();
        if self.names.is_empty() {
            if let Some(name) = Path::new(&self.path).file_name() {
                if let Some(name) = name.to_str() {
                    self.names = vec![name.to_string()];
                }
            }
        }
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub path: Vec<RegItem>,
    #[serde(default)]
    pub is_pull_rebase: bool,
    #[serde(default)]
    pub is_push_rebase: bool,
    #[serde(default)]
    pub grep_app: String,
    #[serde(default)]
    pub edit_app: String,
    #[serde(default)]
    pub debug_print_system: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConfig {
    #[serde(default)]
    path: Vec<RawRegItem>,
    #[serde(default)]
    is_pull_rebase: bool,
    #[serde(default)]
    is_push_rebase: bool,
    #[serde(default)]
    grep_app: Option<String>,
    #[serde(default)]
    edit_app: Option<String>,
    #[serde(default)]
    debug_print_system: bool,
}

#[derive(Debug, Deserialize)]
struct RawRegItem {
    names: NamesField,
    path: String,
    #[serde(default)]
    groups: Vec<String>,
    #[serde(default)]
    repo: bool,
}

impl From<RawRegItem> for RegItem {
    fn from(raw: RawRegItem) -> Self {
        let names = match raw.names {
            NamesField::Single(s) => vec![s],
            NamesField::Many(v) => v,
        };
        RegItem {
            names,
            path: raw.path,
            groups: raw.groups,
            repo: raw.repo,
        }
        .normalized()
    }
}

impl Config {
    pub fn load_or_create() -> anyhow::Result<(Self, PathBuf)> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let base_dir = Path::new(&home).join(".synapcmd");
        if !base_dir.is_dir() {
            fs::create_dir_all(&base_dir)?;
        }

        let cfg_path = base_dir.join("cfg.json");
        let old_path = base_dir.join("path.json");
        if old_path.exists() && !cfg_path.exists() {
            fs::rename(&old_path, &cfg_path)?;
        }

        if !cfg_path.exists() {
            let cfg = Config::default();
            cfg.save(&cfg_path)?;
            return Ok((cfg, cfg_path));
        }

        let text = fs::read_to_string(&cfg_path)?;
        let raw: RawConfig = serde_json::from_str(&text)?;
        let mut cfg = Config {
            path: raw.path.into_iter().map(RegItem::from).collect(),
            is_pull_rebase: raw.is_pull_rebase,
            is_push_rebase: raw.is_push_rebase,
            grep_app: raw.grep_app.unwrap_or_default(),
            edit_app: raw.edit_app.unwrap_or_default(),
            debug_print_system: raw.debug_print_system,
        };
        cfg.ensure_defaults();
        Ok((cfg, cfg_path))
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text)?;
        Ok(())
    }

    fn ensure_defaults(&mut self) {
        if self.grep_app.is_empty() {
            if is_executable_in_path("ag") {
                self.grep_app = "ag".to_string();
            } else if is_executable_in_path("ack") {
                self.grep_app = "ack".to_string();
            } else {
                self.grep_app = "grep".to_string();
            }
        }
        if self.edit_app.is_empty() {
            if is_executable_in_path("code") {
                self.edit_app = "code".to_string();
            } else {
                self.edit_app = "vi".to_string();
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut cfg = Config {
            path: Vec::new(),
            is_pull_rebase: true,
            is_push_rebase: true,
            grep_app: String::new(),
            edit_app: String::new(),
            debug_print_system: false,
        };
        cfg.ensure_defaults();
        cfg
    }
}

