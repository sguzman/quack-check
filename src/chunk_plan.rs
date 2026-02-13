use crate::{config::Config, probe::ProbeResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPlan {
    pub page_count: u32,
    pub chunks: Vec<PageRange>,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRange {
    pub start_page: u32, // 1-based inclusive
    pub end_page: u32,   // 1-based inclusive
}

impl ChunkPlan {
    pub fn from_probe(cfg: &Config, probe: &ProbeResult) -> Result<Self> {
        let page_count = probe.input.page_count;
        Ok(Self::from_page_count(cfg, page_count))
    }

    pub fn single(page_count: u32, strategy: &str) -> ChunkPlan {
        ChunkPlan {
            page_count,
            chunks: vec![PageRange {
                start_page: 1,
                end_page: page_count.max(1),
            }],
            strategy: strategy.to_string(),
        }
    }

    pub fn from_page_count(cfg: &Config, page_count: u32) -> ChunkPlan {
        let target = cfg.chunking.target_pages_per_chunk.max(1);
        let maxp = cfg.chunking.max_pages_per_chunk.max(1);
        let minp = cfg.chunking.min_pages_per_chunk.max(1).min(maxp);

        let mut chunks = Vec::new();
        let mut p = 1u32;

        while p <= page_count {
            let mut end = (p + target - 1).min(page_count);
            let span = end - p + 1;
            if span > maxp {
                end = p + maxp - 1;
            }

            let remaining = page_count.saturating_sub(end);
            if remaining > 0 && remaining < minp && !chunks.is_empty() {
                end = page_count;
            }

            chunks.push(PageRange {
                start_page: p,
                end_page: end,
            });
            p = end + 1;
        }

        ChunkPlan {
            page_count,
            chunks,
            strategy: cfg.chunking.strategy.clone(),
        }
    }
}
