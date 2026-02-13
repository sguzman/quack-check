use crate::{config::Config, engine::Engine};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub input: ProbeInput,
    pub sample: ProbeSampleStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeInput {
    pub path: String,
    pub file_bytes: u64,
    pub page_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSampleStats {
    pub sampled_pages: u32,
    pub avg_chars_per_page: u32,
    pub garbage_ratio: f32,
    pub whitespace_ratio: f32,
}

pub fn probe_pdf(cfg: &Config, engine: &dyn Engine, input: &Path) -> Result<ProbeResult> {
    let meta = std::fs::metadata(input).with_context(|| "stat input")?;
    let file_bytes = meta.len();
    if file_bytes > cfg.limits.max_input_file_bytes {
        anyhow::bail!("input exceeds max_input_file_bytes: {}", file_bytes);
    }

    let probe = engine
        .probe_pdf(input, cfg.classification.sample_pages)
        .with_context(|| "engine probe_pdf failed")?;

    if probe.page_count > cfg.limits.max_input_pages {
        anyhow::bail!("input exceeds max_input_pages: {}", probe.page_count);
    }
    if probe.page_count == 0 {
        anyhow::bail!("input has zero pages");
    }

    Ok(ProbeResult {
        input: ProbeInput {
            path: input.display().to_string(),
            file_bytes,
            page_count: probe.page_count,
        },
        sample: ProbeSampleStats {
            sampled_pages: probe.sampled_pages,
            avg_chars_per_page: probe.avg_chars_per_page,
            garbage_ratio: probe.garbage_ratio,
            whitespace_ratio: probe.whitespace_ratio,
        },
    })
}
