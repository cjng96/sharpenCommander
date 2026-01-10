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
pub struct Config {
    #[serde(default)]
    pub path: Vec<RegItem>,
    #[serde(default)]
    pub isPullRebase: bool,
    #[serde(default)]
    pub isPushRebase: bool,
    #[serde(default)]
    pub grepApp: String,
    #[serde(default)]
    pub editApp: String,
    #[serde(default)]
    pub debugPrintSystem: bool,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    path: Vec<RawRegItem>,
    #[serde(default)]
    isPullRebase: bool,
    #[serde(default)]
    isPushRebase: bool,
    #[serde(default)]
    grepApp: Option<String>,
    #[serde(default)]
    editApp: Option<String>,
    #[serde(default)]
    debugPrintSystem: bool,
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
            isPullRebase: raw.isPullRebase,
            isPushRebase: raw.isPushRebase,
            grepApp: raw.grepApp.unwrap_or_default(),
            editApp: raw.editApp.unwrap_or_default(),
            debugPrintSystem: raw.debugPrintSystem,
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
        if self.grepApp.is_empty() {
            if is_executable_in_path("ag") {
                self.grepApp = "ag".to_string();
            } else if is_executable_in_path("ack") {
                self.grepApp = "ack".to_string();
            } else {
                self.grepApp = "grep".to_string();
            }
        }
        if self.editApp.is_empty() {
            if is_executable_in_path("code") {
                self.editApp = "code".to_string();
            } else {
                self.editApp = "vi".to_string();
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut cfg = Config {
            path: Vec::new(),
            isPullRebase: true,
            isPushRebase: true,
            grepApp: String::new(),
            editApp: String::new(),
            debugPrintSystem: false,
        };
        cfg.ensure_defaults();
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_reg_item() {
        let item = RegItem {
            names: vec![],
            path: "/tmp".to_string(),
            groups: vec![],
            repo: false,
        }
        .normalized();
        assert!(!item.names.is_empty());
    }

    #[test]
    fn deserialize_names_field() {
        let raw = r#"{"path":[{"names":"abc","path":"/tmp"}]}"#;
        let parsed: RawConfig = serde_json::from_str(raw).expect("parse");
        assert_eq!(parsed.path.len(), 1);
    }
}
