use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub global: Global,
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub hashing: Hashing,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub classification: Classification,
    #[serde(default)]
    pub chunking: Chunking,
    #[serde(default)]
    pub engine: Engine,
    #[serde(default)]
    pub native_text: NativeText,
    #[serde(default)]
    pub docling: Docling,
    #[serde(default)]
    pub postprocess: Postprocess,
    #[serde(default)]
    pub output: Output,
    #[serde(default)]
    pub logging: Logging,
    #[serde(default)]
    pub debug: Debug,
    #[serde(default)]
    pub security: Security,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading config: {}", path.display()))?;
        let cfg: Config = toml::from_str(&raw).with_context(|| "parsing TOML")?;
        Ok(cfg)
    }

    /// A stable, normalization-friendly string for hashing.
    pub fn normalized_for_hash(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            global: Default::default(),
            paths: Default::default(),
            hashing: Default::default(),
            limits: Default::default(),
            classification: Default::default(),
            chunking: Default::default(),
            engine: Default::default(),
            native_text: Default::default(),
            docling: Default::default(),
            postprocess: Default::default(),
            output: Default::default(),
            logging: Default::default(),
            debug: Default::default(),
            security: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Global {
    pub job_name: String,
    pub offline_only: bool,
    pub keep_intermediates: bool,
    pub resume: bool,
    pub max_parallel_chunks: usize,
    pub print_summary: bool,
}
impl Default for Global {
    fn default() -> Self {
        Self {
            job_name: "default".into(),
            offline_only: true,
            keep_intermediates: true,
            resume: true,
            max_parallel_chunks: 1,
            print_summary: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paths {
    pub out_dir: String,
    pub work_dir: String,
    pub cache_dir: String,
    pub docling_artifacts_dir: String,
    pub scripts_dir: String,
}
impl Default for Paths {
    fn default() -> Self {
        Self {
            out_dir: "out".into(),
            work_dir: ".quack-check-work".into(),
            cache_dir: ".quack-check-cache".into(),
            docling_artifacts_dir: ".docling-artifacts".into(),
            scripts_dir: "scripts".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hashing {
    pub mode: String,
    pub fast_window_bytes: u64,
}
impl Default for Hashing {
    fn default() -> Self {
        Self {
            mode: "fast_2x16mb".into(),
            fast_window_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    pub max_input_file_bytes: u64,
    pub max_input_pages: u32,
    pub require_chunking_over_pages: u32,
    pub require_chunking_over_bytes: u64,
    pub job_timeout_seconds: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_file_bytes: 2 * 1024 * 1024 * 1024,
            max_input_pages: 20000,
            require_chunking_over_pages: 200,
            require_chunking_over_bytes: 200_000_000,
            job_timeout_seconds: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub sample_pages: u32,
    pub enable_render_probe: bool,
    pub min_avg_chars_per_page_for_high_text: u32,
    pub max_avg_chars_per_page_for_scan: u32,
    pub max_garbage_ratio_for_high_text: f32,
    pub max_whitespace_ratio_for_high_text: f32,
    pub forced_tier: String,
}
impl Default for Classification {
    fn default() -> Self {
        Self {
            sample_pages: 12,
            enable_render_probe: false,
            min_avg_chars_per_page_for_high_text: 1200,
            max_avg_chars_per_page_for_scan: 80,
            max_garbage_ratio_for_high_text: 0.02,
            max_whitespace_ratio_for_high_text: 0.55,
            forced_tier: "AUTO".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunking {
    pub strategy: String,
    pub target_pages_per_chunk: u32,
    pub max_pages_per_chunk: u32,
    pub min_pages_per_chunk: u32,
    pub cap_chunk_bytes: bool,
    pub max_chunk_bytes: u64,
    pub split_backend: String,
    pub keep_split_pdfs: bool,
}
impl Default for Chunking {
    fn default() -> Self {
        Self {
            strategy: "physical_split".into(),
            target_pages_per_chunk: 40,
            max_pages_per_chunk: 80,
            min_pages_per_chunk: 10,
            cap_chunk_bytes: true,
            max_chunk_bytes: 50_000_000,
            split_backend: "python_pypdf".into(),
            keep_split_pdfs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engine {
    pub high_text_engine: String,
    pub mixed_text_engine: String,
    pub scan_engine: String,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            high_text_engine: "native_text".into(),
            mixed_text_engine: "docling".into(),
            scan_engine: "docling".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeText {
    pub backend: String,
    pub normalize_unicode: bool,
    pub collapse_whitespace: bool,
    pub fix_hyphenation: bool,
    pub light_markdown: bool,
}
impl Default for NativeText {
    fn default() -> Self {
        Self {
            backend: "python_pypdf".into(),
            normalize_unicode: true,
            collapse_whitespace: true,
            fix_hyphenation: true,
            light_markdown: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Docling {
    pub python_exe: String,
    pub max_num_pages: u32,
    pub max_file_size_bytes: u64,
    pub raises_on_error: bool,
    pub process_isolation: bool,
    pub chunk_timeout_seconds: u64,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub backend: DoclingBackend,
    #[serde(default)]
    pub pipeline: DoclingPipeline,
    #[serde(default)]
    pub ocr: DoclingOcr,
    #[serde(default)]
    pub accelerator: DoclingAccelerator,
    #[serde(default)]
    pub vlm: DoclingVlm,
}
impl Default for Docling {
    fn default() -> Self {
        Self {
            python_exe: "python3".into(),
            max_num_pages: 1000,
            max_file_size_bytes: 500_000_000,
            raises_on_error: false,
            process_isolation: true,
            chunk_timeout_seconds: 600,
            env: Default::default(),
            backend: Default::default(),
            pipeline: Default::default(),
            ocr: Default::default(),
            accelerator: Default::default(),
            vlm: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoclingBackend {
    pub pdf_backend: String,
}
impl Default for DoclingBackend {
    fn default() -> Self {
        Self {
            pdf_backend: "AUTO".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoclingPipeline {
    pub do_ocr: bool,
    pub force_backend_text: bool,
    pub do_table_structure: bool,
    pub do_code_enrichment: bool,
    pub do_formula_enrichment: bool,
    pub do_picture_description: bool,
    pub do_picture_classification: bool,
    pub generate_page_images: bool,
    pub generate_picture_images: bool,
    pub generate_table_images: bool,
    pub generate_parsed_pages: bool,
    pub create_legacy_output: bool,
    pub document_timeout_seconds: u64,
    pub enable_remote_services: bool,
    pub allow_external_plugins: bool,
    pub use_threaded_pipeline: bool,
    pub num_threads: u32,
    pub queue_max_size: u32,
    pub layout_batch_size: u32,
    pub table_batch_size: u32,
    pub picture_batch_size: u32,
    pub page_batch_size: u32,
    pub images_scale: f32,
}
impl Default for DoclingPipeline {
    fn default() -> Self {
        Self {
            do_ocr: false,
            force_backend_text: false,
            do_table_structure: true,
            do_code_enrichment: false,
            do_formula_enrichment: false,
            do_picture_description: false,
            do_picture_classification: false,
            generate_page_images: false,
            generate_picture_images: false,
            generate_table_images: false,
            generate_parsed_pages: false,
            create_legacy_output: false,
            document_timeout_seconds: 0,
            enable_remote_services: false,
            allow_external_plugins: false,
            use_threaded_pipeline: true,
            num_threads: 4,
            queue_max_size: 8,
            layout_batch_size: 16,
            table_batch_size: 8,
            picture_batch_size: 4,
            page_batch_size: 8,
            images_scale: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoclingOcr {
    pub engine: String,
    pub langs: Vec<String>,
    pub force_full_page_ocr: bool,
    pub bitmap_area_threshold: f32,
    pub force_ocr: bool,
    pub tesseract_cli_args: String,
}
impl Default for DoclingOcr {
    fn default() -> Self {
        Self {
            engine: "easyocr".into(),
            langs: vec!["en".into()],
            force_full_page_ocr: false,
            bitmap_area_threshold: 0.25,
            force_ocr: false,
            tesseract_cli_args: "".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoclingAccelerator {
    pub device: String,
    pub inference_threads: u32,
    pub use_fp16: bool,
}
impl Default for DoclingAccelerator {
    fn default() -> Self {
        Self {
            device: "AUTO".into(),
            inference_threads: 0,
            use_fp16: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoclingVlm {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub api_key_env: String,
    pub force_backend_text: bool,
}
impl Default for DoclingVlm {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".into(),
            model: "".into(),
            api_key_env: "OPENAI_API_KEY".into(),
            force_backend_text: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Postprocess {
    pub normalize_unicode: bool,
    pub normalize_newlines: bool,
    pub trim_trailing_whitespace: bool,
    pub remove_repeated_lines: bool,
    pub repeated_line_min_occurrences: u32,
    pub repeated_line_max_length: u32,
    pub remove_by_regex: bool,
    #[serde(default)]
    pub regex: PostprocessRegex,
}
impl Default for Postprocess {
    fn default() -> Self {
        Self {
            normalize_unicode: true,
            normalize_newlines: true,
            trim_trailing_whitespace: true,
            remove_repeated_lines: true,
            repeated_line_min_occurrences: 6,
            repeated_line_max_length: 120,
            remove_by_regex: true,
            regex: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostprocessRegex {
    pub patterns: Vec<String>,
}
impl Default for PostprocessRegex {
    fn default() -> Self {
        Self {
            patterns: vec![
                "^(page\\s+\\d+|\\d+\\s*/\\s*\\d+)$".into(),
                "^[A-Z0-9\\s\\-]{12,}$".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub write_markdown: bool,
    pub write_text: bool,
    pub write_report_json: bool,
    pub write_chunk_json: bool,
    pub markdown_filename: String,
    pub text_filename: String,
    pub report_filename: String,
    pub write_index_json: bool,
}
impl Default for Output {
    fn default() -> Self {
        Self {
            write_markdown: true,
            write_text: true,
            write_report_json: true,
            write_chunk_json: true,
            markdown_filename: "transcript.md".into(),
            text_filename: "transcript.txt".into(),
            report_filename: "report.json".into(),
            write_index_json: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    pub level: String,
    pub json: bool,
    pub write_to_file: bool,
    pub file_path: String,
}
impl Default for Logging {
    fn default() -> Self {
        Self {
            level: "info".into(),
            json: false,
            write_to_file: true,
            file_path: "".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Debug {
    pub keep_python_stderr: bool,
    pub dump_effective_config: bool,
}
impl Default for Debug {
    fn default() -> Self {
        Self {
            keep_python_stderr: true,
            dump_effective_config: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Security {
    pub reject_url_inputs: bool,
    pub pin_scripts_dir: bool,
}
impl Default for Security {
    fn default() -> Self {
        Self {
            reject_url_inputs: true,
            pin_scripts_dir: true,
        }
    }
}
