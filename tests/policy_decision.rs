use quack_check::{
    config::Config,
    policy::{decide, QualityTier},
    probe::{ProbeInput, ProbeResult, ProbeSampleStats},
};

fn mk_probe(avg: u32, garbage: f32, ws: f32, pages: u32) -> ProbeResult {
    ProbeResult {
        input: ProbeInput {
            path: "x.pdf".into(),
            file_bytes: 1,
            page_count: pages,
        },
        sample: ProbeSampleStats {
            sampled_pages: 10,
            avg_chars_per_page: avg,
            garbage_ratio: garbage,
            whitespace_ratio: ws,
        },
    }
}

#[test]
fn high_text_classification() {
    let cfg = Config::default();
    let p = mk_probe(5000, 0.0, 0.2, 300);
    let d = decide(&cfg, &p);
    assert!(matches!(d.tier, QualityTier::HighText));
}

#[test]
fn scan_classification() {
    let cfg = Config::default();
    let p = mk_probe(10, 0.0, 0.1, 50);
    let d = decide(&cfg, &p);
    assert!(matches!(d.tier, QualityTier::Scan));
    assert!(d.do_ocr);
}
