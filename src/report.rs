use crate::{
    policy::PolicyDecision,
    probe::{ProbeInput, ProbeSampleStats},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobReport {
    pub input: ProbeInput,
    pub sample: ProbeSampleStats,
    pub decision: PolicyDecision,
    pub chunk_reports: Vec<ChunkReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkReport {
    pub chunk_index: u32,
    pub start_page: u32,
    pub end_page: u32,
    pub ok: bool,
    pub warnings: Vec<String>,
    pub meta: serde_json::Value,
}
