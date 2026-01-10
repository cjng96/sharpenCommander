use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run_shell(cmd: &str) -> std::io::Result<Output> {
    Command::new("sh").arg("-c").arg(cmd).output()
}

pub fn system(cmd: &str) -> std::io::Result<String> {
    let out = run_shell(cmd)?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if !err.trim().is_empty() {
            text.push_str(&err);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            text.trim().to_string(),
        ));
    }
    Ok(text.trim_end_matches(['\r', '\n', ' ']).to_string())
}

pub fn system_safe(cmd: &str) -> (String, i32) {
    match run_shell(cmd) {
        Ok(out) => {
            let code = out.status.code().unwrap_or(1);
            let mut text = String::from_utf8_lossy(&out.stdout).to_string();
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                if !err.trim().is_empty() {
                    text.push_str(&err);
                }
            }
            (text.trim_end_matches(['\r', '\n', ' ']).to_string(), code)
        }
        Err(err) => (err.to_string(), 1),
    }
}

pub fn system_ret(cmd: &str) -> i32 {
    run_shell(cmd)
        .ok()
        .and_then(|out| out.status.code())
        .unwrap_or(1)
}

pub fn system_stream(cmd: &str) -> std::io::Result<i32> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    Ok(status.code().unwrap_or(1))
}

pub fn program_path(sub: Option<&str>) -> std::io::Result<PathBuf> {
    let mut exe = std::env::current_exe()?;
    exe.pop();
    if let Some(sub) = sub {
        Ok(exe.join(sub))
    } else {
        Ok(exe)
    }
}

pub fn is_executable_in_path(name: &str) -> bool {
    which::which(name).is_ok()
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home).join(stripped);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_output() {
        let out = system("printf 'hi\\n'").expect("run");
        assert_eq!(out, "hi");
    }

    #[test]
    fn expand_tilde_keeps_absolute() {
        let p = expand_tilde("/tmp/test");
        assert_eq!(p, PathBuf::from("/tmp/test"));
    }
}
