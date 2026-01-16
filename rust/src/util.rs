use std::path::{Path, PathBuf};

pub fn unwrap_quotes_filename(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].replace("\\\"", "\"")
    } else {
        trimmed.to_string()
    }
}

pub fn strip_ansi(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            // Check for [
            if let Some('[') = chars.peek() {
                chars.next();
                // Consume until 'm' or other termination
                while let Some(c) = chars.next() {
                    if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') {
                        break;
                    }
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

pub fn match_disorder(input: &str, filters: &[String]) -> bool {
    let mut text = input.to_string();
    for f in filters {
        if let Some(pos) = text.find(f) {
            let end = pos + f.len();
            text.replace_range(pos..end, "");
        } else {
            return false;
        }
    }
    true
}

pub fn match_disorder_count(input: &str, filters: &[String]) -> usize {
    let mut text = input.to_string();
    let mut count = 0;
    for f in filters {
        if let Some(pos) = text.find(f) {
            let end = pos + f.len();
            text.replace_range(pos..end, "");
            count += 1;
        }
    }
    count
}

pub fn walk_dirs(root: &Path, ignore: &[&str], limit: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(read_dir) = std::fs::read_dir(&dir) {
            for entry in read_dir.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let name = entry.file_name();
                        if let Some(name_str) = name.to_str() {
                            if ignore.contains(&name_str) {
                                continue;
                            }
                        }
                        let path = entry.path();
                        if let Ok(rel) = path.strip_prefix(root) {
                            out.push(rel.to_path_buf());
                        }
                        if out.len() >= limit {
                            return out;
                        }
                        stack.push(path);
                    }
                }
            }
        }
        if out.len() >= limit {
            break;
        }
    }
    out
}

