use std::path::{Path, PathBuf};

pub fn unwrap_quotes_filename(input: &str) -> String {
    if input.starts_with('"') && input.ends_with('"') && input.len() >= 2 {
        input[1..input.len() - 1].replace('"', "\\\"")
    } else {
        input.to_string()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_disorder_basic() {
        let filters = vec!["ab".to_string(), "cd".to_string()];
        assert!(match_disorder("xxabyycdzz", &filters));
        assert!(!match_disorder("abyyzz", &filters));
    }

    #[test]
    fn match_disorder_count_partial() {
        let filters = vec!["aa".to_string(), "bb".to_string()];
        assert_eq!(match_disorder_count("aabbcc", &filters), 2);
        assert_eq!(match_disorder_count("aacc", &filters), 1);
    }
}
