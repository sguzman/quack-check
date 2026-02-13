use quack_check::{config::Config, postprocess::merge_markdown};

#[test]
fn removes_repeated_lines() {
    let mut cfg = Config::default();
    cfg.postprocess.remove_repeated_lines = true;
    cfg.postprocess.repeated_line_min_occurrences = 3;

    let parts = vec![
        "BOOK TITLE\nHello\nPage 1".to_string(),
        "BOOK TITLE\nWorld\nPage 2".to_string(),
        "BOOK TITLE\nAgain\nPage 3".to_string(),
    ];

    let merged = merge_markdown(&cfg, parts).unwrap();
    assert!(!merged.contains("BOOK TITLE"));
}
