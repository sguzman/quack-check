use quack_check::config::Config;

#[test]
fn parse_example_config() {
    let raw = include_str!("../quack-check.example.toml");
    let cfg: Config = toml::from_str(raw).expect("parse TOML");
    assert!(cfg.global.max_parallel_chunks >= 1);
    assert!(!cfg.paths.out_dir.is_empty());
}
