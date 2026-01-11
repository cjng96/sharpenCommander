use std::fs::OpenOptions;
use std::io::Write;
use std::panic::Location;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn app_log_raw(line: &str) {
    let path = program_path(None).unwrap_or_else(|_| PathBuf::from(".")).join("app.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", ts, line);
    }
}

fn app_log_raw_with_loc(loc: &Location<'_>, line: &str) {
    app_log_raw(&format!("{}:{} {}", loc.file(), loc.line(), line));
}

#[track_caller]
pub fn app_log(line: &str) {
    let loc = Location::caller();
    app_log_raw_with_loc(loc, line);
}

fn app_log_block_with_loc(loc: &Location<'_>, prefix: &str, content: &str) {
    if content.is_empty() {
        app_log_raw_with_loc(loc, &format!("{} <empty>", prefix));
        return;
    }
    app_log_raw_with_loc(loc, prefix);
    for line in content.lines() {
        app_log_raw_with_loc(loc, &format!("  {}", line));
    }
}

fn log_command_result_at(
    loc: &Location<'_>,
    context: &str,
    cmd: &str,
    result: &std::io::Result<String>,
) {
    app_log_raw_with_loc(loc, &format!("[{}] CMD: {}", context, cmd));
    match result {
        Ok(out) => app_log_block_with_loc(loc, &format!("[{}] OUT", context), out),
        Err(err) => app_log_block_with_loc(loc, &format!("[{}] ERR", context), &err.to_string()),
    }
}

#[track_caller]
pub fn log_command_result(context: &str, cmd: &str, result: &std::io::Result<String>) {
    let loc = Location::caller();
    log_command_result_at(loc, context, cmd, result);
}

#[track_caller]
pub fn system_logged(context: &str, cmd: &str) -> std::io::Result<String> {
    let loc = Location::caller();
    let result = system(cmd);
    log_command_result_at(loc, context, cmd, &result);
    result
}

pub fn system_ret(cmd: &str) -> i32 {
    run_shell(cmd)
        .ok()
        .and_then(|out| out.status.code())
        .unwrap_or(1)
}

pub fn system_stream(cmd: &str) -> std::io::Result<i32> {
    app_log(&format!("system_stream: {}", cmd));

    let stdin = std::fs::File::open("/dev/tty")
        .map(Stdio::from)
        .unwrap_or_else(|e| {
            app_log(&format!("Failed to open /dev/tty for stdin: {}", e));
            Stdio::inherit()
        });

    let stdout = OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .map(Stdio::from)
        .unwrap_or_else(|e| {
            app_log(&format!("Failed to open /dev/tty for stdout: {}", e));
            Stdio::inherit()
        });

    let stderr = OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .map(Stdio::from)
        .unwrap_or_else(|e| {
            app_log(&format!("Failed to open /dev/tty for stderr: {}", e));
            Stdio::inherit()
        });

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()?;

    let status = child.wait()?;
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
