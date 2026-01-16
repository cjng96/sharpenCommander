use sc::util::*;
use sc::system::*;
use sc::config::*;
use sc::app::*;
use std::path::PathBuf;

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

#[test]
fn test_strip_ansi() {
    let input = "\x1b[31mred\x1b[0m text";
    assert_eq!(strip_ansi(input), "red text");
}

#[test]
fn test_unwrap_quotes_filename() {
    assert_eq!(unwrap_quotes_filename("\"file name\""), "file name");
    assert_eq!(unwrap_quotes_filename("\"file\\\"name\""), "file\"name");
    assert_eq!(unwrap_quotes_filename("plain_name"), "plain_name");
}

#[test]
fn expand_tilde_keeps_absolute() {
    let p = expand_tilde("/tmp/test");
    assert_eq!(p, PathBuf::from("/tmp/test"));
}

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
fn test_reg_find_by_name() {
    let mut ctx = AppContext {
        config: Config::default(),
        config_path: PathBuf::from("fake"),
    };
    ctx.config.path.push(RegItem {
        names: vec!["MyRepo".to_string()],
        path: "/path/to/repo".to_string(),
        groups: vec![],
        repo: true,
    });

    assert!(ctx.reg_find_by_name("myrepo").is_ok());
    assert!(ctx.reg_find_by_name("OTHER").is_err());
}

#[test]
fn test_calculate_goto_score() {
    let fragments = vec!["abc".to_string()];
    let filter = "abc";
    
    let exact = calculate_goto_score("abc", filter, &fragments);
    let starts = calculate_goto_score("abcd", filter, &fragments);
    let contains = calculate_goto_score("xabcy", filter, &fragments);
    let none = calculate_goto_score("def", filter, &fragments);
    
    assert!(exact > starts);
    assert!(starts > contains);
    assert!(contains > none);
    assert_eq!(none, 0);
}

#[test]
fn test_app_context_save_path() {
    let ctx = AppContext {
        config: Config::default(),
        config_path: PathBuf::from("fake"),
    };
    
    // We can't easily test /tmp/ write in all environments, but we can verify the function exists and runs
    let res = ctx.save_path("/tmp/test_path");
    assert!(res.is_ok());
    
    let saved = std::fs::read_to_string("/tmp/cmdDevTool.path").unwrap();
    assert_eq!(saved, "/tmp/test_path");
}

