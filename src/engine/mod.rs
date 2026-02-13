pub mod python;
pub mod types;

use anyhow::Result;
use std::path::Path;

pub use types::{ConvertIn, ConvertOut, DocDiag, ProbeOut, SplitChunk};

pub trait Engine {
    fn doctor(&self) -> Result<DocDiag>;
    fn probe_pdf(&self, input: &Path, sample_pages: u32) -> Result<ProbeOut>;
    fn split_pdf(&self, input: &Path, out_dir: &Path, ranges: &[crate::chunk_plan::PageRange])
        -> Result<Vec<SplitChunk>>;
    fn convert_docling(&self, req: &ConvertIn) -> Result<ConvertOut>;
    fn convert_native_text(&self, req: &ConvertIn) -> Result<ConvertOut>;
}
