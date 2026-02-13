use crate::{
    config::Config,
    engine::{python::PythonEngine, Engine},
    pipeline::Pipeline,
    util::{ensure_dir, now_rfc3339, sha256_hex},
};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[derive(Parser, Debug)]
#[command(name = "quack-check")]
#[command(about = "Deterministic PDF transcript orchestrator (Docling + chunking + policy)")]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Command,

    /// Path to config TOML. If omitted, uses ./quack-check.toml if present.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Override log level (trace/debug/info/warn/error).
    #[arg(long)]
    pub log_level: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Doctor {},
    Classify {
        #[arg(long)]
        input: PathBuf,
    },
    Plan {
        #[arg(long)]
        input: PathBuf,
    },
    Run {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
}

pub fn dispatch(args: Args) -> Result<()> {
    let cfg_path = resolve_config_path(args.config.as_deref())?;
    let cfg = Config::load(&cfg_path)?;

    match &args.cmd {
        Command::Doctor {} => {
            let log_path = resolve_log_path(&cfg, None);
            let _guard = init_logging(&args, &cfg, log_path.as_deref())?;
            doctor(&cfg)
        }
        Command::Classify { input } => {
            let log_path = resolve_log_path(&cfg, None);
            let _guard = init_logging(&args, &cfg, log_path.as_deref())?;
            classify(&cfg, input)
        }
        Command::Plan { input } => {
            let log_path = resolve_log_path(&cfg, None);
            let _guard = init_logging(&args, &cfg, log_path.as_deref())?;
            plan(&cfg, input)
        }
        Command::Run { input, out_dir } => run(&args, &cfg, input, out_dir.as_deref()),
    }
}

fn resolve_config_path(user: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = user {
        return Ok(p.to_path_buf());
    }
    let default = PathBuf::from("quack-check.toml");
    if default.exists() {
        Ok(default)
    } else {
        Ok(PathBuf::from("quack-check.example.toml"))
    }
}

fn init_logging(args: &Args, cfg: &Config, file_path: Option<&Path>) -> Result<Option<WorkerGuard>> {
    let level = args
        .log_level
        .as_deref()
        .unwrap_or(cfg.logging.level.as_str());

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let stdout_layer = if cfg.logging.json {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .boxed()
    };

    let (file_layer, guard) = if let Some(path) = file_path {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        ensure_dir(parent)?;
        let file = std::fs::File::create(path)
            .with_context(|| format!("create log file: {}", path.display()))?;
        let (non_blocking, guard) = tracing_appender::non_blocking(file);
        let layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .boxed();
        (Some(layer), Some(guard))
    } else {
        (None, None)
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .try_init()
        .map_err(|e| anyhow!("failed to init logging: {e}"))?;

    Ok(guard)
}

fn doctor(cfg: &Config) -> Result<()> {
    let engine = PythonEngine::new(cfg)?;
    let diag = engine.doctor()?;
    println!("{}", serde_json::to_string_pretty(&diag)?);
    Ok(())
}

fn classify(cfg: &Config, input: &Path) -> Result<()> {
    let engine = PythonEngine::new(cfg)?;
    let probe = crate::probe::probe_pdf(cfg, &engine, input)?;
    let decision = crate::policy::decide(cfg, &probe);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "input": input,
            "probe": probe,
            "decision": decision,
        }))?
    );
    Ok(())
}

fn plan(cfg: &Config, input: &Path) -> Result<()> {
    let engine = PythonEngine::new(cfg)?;
    let probe = crate::probe::probe_pdf(cfg, &engine, input)?;
    let plan = crate::chunk_plan::ChunkPlan::from_probe(cfg, &probe)?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

fn run(args: &Args, cfg: &Config, input: &Path, out_override: Option<&Path>) -> Result<()> {
    validate_input(cfg, input)?;

    let cfg_norm = cfg.normalized_for_hash();
    let cfg_hash = sha256_hex(cfg_norm.as_bytes());
    let input_hash = crate::util::hash_file(cfg, input)
        .with_context(|| format!("hashing input: {}", input.display()))?;
    let job_id = sha256_hex(format!("{}:{}", cfg_hash, input_hash).as_bytes());

    let out_root = out_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&cfg.paths.out_dir));
    let job_dir = out_root.join(&job_id);

    if job_dir.exists() && !cfg.global.resume {
        return Err(anyhow!(
            "job_dir already exists and resume=false: {}",
            job_dir.display()
        ));
    }

    ensure_dir(&job_dir)?;
    ensure_dir(&job_dir.join("final"))?;
    ensure_dir(&job_dir.join("logs"))?;
    ensure_dir(&job_dir.join("chunks"))?;

    let log_path = resolve_log_path(cfg, Some(&job_dir));
    let _guard = init_logging(args, cfg, log_path.as_deref())?;

    info!("job_id={job_id} out={}", job_dir.display());

    if cfg.debug.dump_effective_config {
        let raw = toml::to_string(cfg).unwrap_or_default();
        std::fs::write(job_dir.join("effective-config.toml"), raw)?;
    }

    ensure_dir(Path::new(&cfg.paths.work_dir))?;
    ensure_dir(Path::new(&cfg.paths.cache_dir))?;
    ensure_dir(Path::new(&cfg.paths.docling_artifacts_dir))?;

    let engine = PythonEngine::new(cfg)?;
    let pipeline = Pipeline::new(cfg, engine);

    let started = now_rfc3339();
    let result = pipeline.run_job(input, &job_dir)?;

    if cfg.output.write_markdown {
        std::fs::write(
            job_dir.join("final").join(&cfg.output.markdown_filename),
            &result.markdown,
        )?;
    }

    if cfg.output.write_text {
        std::fs::write(
            job_dir.join("final").join(&cfg.output.text_filename),
            &result.text,
        )?;
    }

    if cfg.output.write_report_json {
        std::fs::write(
            job_dir.join("final").join(&cfg.output.report_filename),
            serde_json::to_string_pretty(&result.report)?,
        )?;
    }

    if cfg.output.write_index_json {
        let index = serde_json::json!({
            "job_id": job_id,
            "started": started,
            "finished": now_rfc3339(),
            "final_markdown": format!("final/{}", cfg.output.markdown_filename),
            "final_text": format!("final/{}", cfg.output.text_filename),
            "report": format!("final/{}", cfg.output.report_filename),
        });
        std::fs::write(job_dir.join("index.json"), serde_json::to_string_pretty(&index)?)?;
    }

    if cfg.global.print_summary {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "job_id": job_id,
                "job_dir": job_dir,
                "status": "ok"
            }))?
        );
    }

    Ok(())
}

fn validate_input(cfg: &Config, input: &Path) -> Result<()> {
    let input_str = input.display().to_string();

    if cfg.security.reject_url_inputs && looks_like_url(&input_str) {
        return Err(anyhow!("URL inputs are disabled: {input_str}"));
    }

    if !input.exists() {
        return Err(anyhow!("input does not exist: {}", input.display()));
    }

    if let Some(ext) = input.extension().and_then(|s| s.to_str()) {
        if ext.to_ascii_lowercase() != "pdf" {
            return Err(anyhow!("input is not a PDF: {}", input.display()));
        }
    } else {
        warn!("input has no extension; assuming PDF: {}", input.display());
    }

    Ok(())
}

fn looks_like_url(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("file://")
}

fn resolve_log_path(cfg: &Config, job_dir: Option<&Path>) -> Option<PathBuf> {
    if !cfg.logging.write_to_file {
        return None;
    }

    if !cfg.logging.file_path.is_empty() {
        return Some(PathBuf::from(&cfg.logging.file_path));
    }

    if let Some(job_dir) = job_dir {
        return Some(job_dir.join("logs").join("quack-check.log"));
    }

    Some(PathBuf::from(&cfg.paths.out_dir).join("quack-check.log"))
}
