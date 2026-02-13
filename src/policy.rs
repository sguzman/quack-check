use crate::{config::Config, probe::ProbeResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityTier {
    HighText,
    MixedText,
    Scan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub tier: QualityTier,
    pub chosen_engine: String,
    pub do_ocr: bool,
}

pub fn decide(cfg: &Config, probe: &ProbeResult) -> PolicyDecision {
    if cfg.classification.forced_tier != "AUTO" {
        return forced(cfg);
    }

    let avg = probe.sample.avg_chars_per_page;
    let garbage = probe.sample.garbage_ratio;
    let ws = probe.sample.whitespace_ratio;

    let tier = if avg >= cfg.classification.min_avg_chars_per_page_for_high_text
        && garbage <= cfg.classification.max_garbage_ratio_for_high_text
        && ws <= cfg.classification.max_whitespace_ratio_for_high_text
    {
        QualityTier::HighText
    } else if avg <= cfg.classification.max_avg_chars_per_page_for_scan {
        QualityTier::Scan
    } else {
        QualityTier::MixedText
    };

    match tier {
        QualityTier::HighText => PolicyDecision {
            tier,
            chosen_engine: cfg.engine.high_text_engine.clone(),
            do_ocr: false,
        },
        QualityTier::MixedText => PolicyDecision {
            tier,
            chosen_engine: cfg.engine.mixed_text_engine.clone(),
            do_ocr: cfg.docling.pipeline.do_ocr,
        },
        QualityTier::Scan => PolicyDecision {
            tier,
            chosen_engine: cfg.engine.scan_engine.clone(),
            do_ocr: true,
        },
    }
}

fn forced(cfg: &Config) -> PolicyDecision {
    let tier = match cfg.classification.forced_tier.as_str() {
        "HIGH_TEXT" => QualityTier::HighText,
        "MIXED_TEXT" => QualityTier::MixedText,
        "SCAN" => QualityTier::Scan,
        _ => QualityTier::MixedText,
    };

    match tier {
        QualityTier::HighText => PolicyDecision {
            tier,
            chosen_engine: cfg.engine.high_text_engine.clone(),
            do_ocr: false,
        },
        QualityTier::MixedText => PolicyDecision {
            tier,
            chosen_engine: cfg.engine.mixed_text_engine.clone(),
            do_ocr: cfg.docling.pipeline.do_ocr,
        },
        QualityTier::Scan => PolicyDecision {
            tier,
            chosen_engine: cfg.engine.scan_engine.clone(),
            do_ocr: true,
        },
    }
}
