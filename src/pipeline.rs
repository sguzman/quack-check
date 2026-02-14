use crate::{
    chunk_plan::ChunkPlan,
    config::Config,
    engine::{ConvertIn, Engine},
    policy,
    postprocess,
    probe,
    report::{ChunkReport, JobReport},
    util::ensure_dir,
};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, info, warn};

pub struct Pipeline<E: Engine> {
    cfg: Config,
    engine: E,
}

pub struct JobOutput {
    pub markdown: String,
    pub text: String,
    pub report: JobReport,
}

impl<E: Engine> Pipeline<E> {
    pub fn new(cfg: &Config, engine: E) -> Self {
        Self {
            cfg: cfg.clone(),
            engine,
        }
    }

    pub fn run_job(&self, input: &Path, job_dir: &Path) -> Result<JobOutput> {
        let started = Instant::now();

        let probe_res = probe::probe_pdf(&self.cfg, &self.engine, input)?;
        let decision = policy::decide(&self.cfg, &probe_res);
        let mut plan = ChunkPlan::from_probe(&self.cfg, &probe_res)?;

        info!(
            "probe page_count={} file_bytes={} avg_chars={} garbage_ratio={} whitespace_ratio={}",
            probe_res.input.page_count,
            probe_res.input.file_bytes,
            probe_res.sample.avg_chars_per_page,
            probe_res.sample.garbage_ratio,
            probe_res.sample.whitespace_ratio
        );
        info!(
            "policy tier={:?} engine={} do_ocr={}",
            decision.tier, decision.chosen_engine, decision.do_ocr
        );
        debug!(?plan, "chunk plan");

        if decision.chosen_engine == "native_text" && self.cfg.native_text.backend != "python_pypdf"
        {
            return Err(anyhow!(
                "unsupported native_text.backend: {}",
                self.cfg.native_text.backend
            ));
        }

        let require_chunking = probe_res.input.page_count > self.cfg.limits.require_chunking_over_pages
            || probe_res.input.file_bytes > self.cfg.limits.require_chunking_over_bytes;

        if !require_chunking && plan.chunks.len() > 1 {
            plan = ChunkPlan::single(plan.page_count, &self.cfg.chunking.strategy);
        }

        if self.cfg.global.max_parallel_chunks > 1 {
            warn!(
                "max_parallel_chunks > 1 is configured, but pipeline runs sequentially in this build"
            );
        }

        let chunks_dir = job_dir.join("chunks");
        ensure_dir(&chunks_dir)?;

        let chunk_inputs = match self.prepare_chunks(input, &plan, &chunks_dir) {
            Ok(inputs) => inputs,
            Err(err) => {
                if self.cfg.chunking.strategy == "physical_split" {
                    warn!("physical split failed; falling back to page_range: {err}");
                    let mut fallback = plan.clone();
                    fallback.strategy = "page_range".to_string();
                    self.prepare_chunks(input, &fallback, &chunks_dir)?
                } else {
                    return Err(err);
                }
            }
        };

        let mut chunk_reports = Vec::new();
        let mut markdown_parts = Vec::new();

        for (i, ch) in chunk_inputs.iter().enumerate() {
            if self.cfg.limits.job_timeout_seconds > 0
                && started.elapsed().as_secs() > self.cfg.limits.job_timeout_seconds
            {
                return Err(anyhow!(
                    "job timeout exceeded: {}s",
                    self.cfg.limits.job_timeout_seconds
                ));
            }

            info!(
                "chunk {} pages {}-{} input={}",
                i,
                ch.start_page,
                ch.end_page,
                ch.input_pdf.display()
            );

            let req = ConvertIn {
                input_pdf: ch.input_pdf.display().to_string(),
                out_dir: chunks_dir.display().to_string(),
                chunk_index: i as u32,
                start_page: ch.start_page,
                end_page: ch.end_page,
                do_ocr: decision.do_ocr,
                pdf_backend: self.cfg.docling.backend.pdf_backend.clone(),
                use_page_range: ch.use_page_range,
            };

            let mut used_fallback = false;
            let mut out = match decision.chosen_engine.as_str() {
                "docling" => self.engine.convert_docling(&req),
                "native_text" => self.engine.convert_native_text(&req),
                other => Err(anyhow!("unknown engine: {other}")),
            };

            if matches!(decision.chosen_engine.as_str(), "native_text") {
                let needs_fallback = match &out {
                    Ok(o) => !o.ok
                        || o.warnings.iter().any(|w| w.contains("missing pypdf import")),
                    Err(e) => e.to_string().contains("missing pypdf import"),
                };

                if needs_fallback {
                    warn!("native_text failed; falling back to docling for chunk {}", i);
                    out = self.engine.convert_docling(&req);
                    used_fallback = true;
                }
            }

            let mut out = out.with_context(|| format!("convert failed for chunk {}", i))?;

            if !out.ok {
                return Err(anyhow!("chunk {} failed; warnings={:?}", i, out.warnings));
            }

            if used_fallback {
                out.warnings
                    .push("native_text failed; fell back to docling".to_string());
            }

            if self.cfg.output.write_chunk_json {
                let chunk_json_path = chunks_dir.join(format!("chunk_{:05}.json", i));
                std::fs::write(&chunk_json_path, serde_json::to_string_pretty(&out)?)?;
            }

            chunk_reports.push(ChunkReport {
                chunk_index: i as u32,
                start_page: ch.start_page,
                end_page: ch.end_page,
                ok: out.ok,
                warnings: out.warnings.clone(),
                meta: out.meta.clone(),
            });

            markdown_parts.push(out.markdown);
        }

        let merged_md = postprocess::merge_markdown(&self.cfg, markdown_parts)?;
        let merged_txt = postprocess::markdown_to_text(&self.cfg, &merged_md)?;

        if !self.cfg.global.keep_intermediates {
            self.cleanup_intermediates(&chunk_inputs)?;
        }

        let report = JobReport {
            input: probe_res.input,
            sample: probe_res.sample,
            decision,
            chunk_reports,
        };

        Ok(JobOutput {
            markdown: merged_md,
            text: merged_txt,
            report,
        })
    }

    fn prepare_chunks(
        &self,
        input: &Path,
        plan: &ChunkPlan,
        chunks_dir: &Path,
    ) -> Result<Vec<ChunkInput>> {
        // Use the plan's strategy so callers can switch strategies for fallback.
        let strategy = plan.strategy.as_str();
        if strategy == "physical_split" && plan.chunks.len() > 1 {
            let split_outputs = self
                .engine
                .split_pdf(input, chunks_dir, &plan.chunks)?;
            let mut out = Vec::new();
            for c in split_outputs {
                let path = PathBuf::from(c.path);
                if self.cfg.chunking.cap_chunk_bytes && self.cfg.chunking.max_chunk_bytes > 0 {
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if meta.len() > self.cfg.chunking.max_chunk_bytes {
                            warn!(
                                "chunk {} exceeds max_chunk_bytes ({} > {})",
                                c.chunk_index,
                                meta.len(),
                                self.cfg.chunking.max_chunk_bytes
                            );
                        }
                    }
                }
                out.push(ChunkInput {
                    input_pdf: path,
                    start_page: c.start_page,
                    end_page: c.end_page,
                    use_page_range: false,
                    temp_file: true,
                });
            }
            return Ok(out);
        }

        let use_page_range = strategy == "page_range" && plan.chunks.len() > 1;
        Ok(plan
            .chunks
            .iter()
            .map(|r| ChunkInput {
                input_pdf: input.to_path_buf(),
                start_page: r.start_page,
                end_page: r.end_page,
                use_page_range,
                temp_file: false,
            })
            .collect())
    }

    fn cleanup_intermediates(&self, chunks: &[ChunkInput]) -> Result<()> {
        if self.cfg.chunking.keep_split_pdfs {
            return Ok(());
        }
        for ch in chunks {
            if ch.temp_file {
                let _ = std::fs::remove_file(&ch.input_pdf);
            }
        }
        Ok(())
    }
}

struct ChunkInput {
    input_pdf: PathBuf,
    start_page: u32,
    end_page: u32,
    use_page_range: bool,
    temp_file: bool,
}
