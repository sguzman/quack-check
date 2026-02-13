use super::{types::*, Engine};
use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

pub struct PythonEngine {
    cfg: Config,
    scripts_dir: PathBuf,
}

impl PythonEngine {
    pub fn new(cfg: &Config) -> Result<Self> {
        let scripts_dir = PathBuf::from(&cfg.paths.scripts_dir);
        if cfg.security.pin_scripts_dir {
            let cwd = std::env::current_dir().with_context(|| "current_dir")?;
            let canon = scripts_dir
                .canonicalize()
                .with_context(|| format!("canonicalize scripts_dir: {}", scripts_dir.display()))?;
            if !canon.starts_with(&cwd) {
                return Err(anyhow!(
                    "scripts_dir is outside cwd while pin_scripts_dir=true: {}",
                    canon.display()
                ));
            }
        }
        for script in [
            "docling_runner.py",
            "pdf_probe.py",
            "pdf_split.py",
            "pdf_text.py",
        ] {
            let path = scripts_dir.join(script);
            if !path.exists() {
                return Err(anyhow!("missing script: {}", path.display()));
            }
        }
        Ok(Self {
            cfg: cfg.clone(),
            scripts_dir,
        })
    }

    fn script(&self, name: &str) -> PathBuf {
        self.scripts_dir.join(name)
    }

    fn run_json<I: serde::Serialize, O: for<'de> serde::Deserialize<'de>>(
        &self,
        script: &Path,
        input: &I,
        timeout_seconds: Option<u64>,
        extra_env: &[(&str, &str)],
    ) -> Result<O> {
        debug!(
            "python run {} timeout={:?}",
            script.display(),
            timeout_seconds
        );
        let mut cmd = Command::new(&self.cfg.docling.python_exe);
        cmd.arg(script);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        for (k, v) in &self.cfg.docling.env {
            cmd.env(k, v);
        }
        for (k, v) in extra_env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawning python: {}", script.display()))?;

        {
            let stdin = child.stdin.as_mut().ok_or_else(|| anyhow!("no stdin"))?;
            let bytes = serde_json::to_vec(input)?;
            use std::io::Write;
            stdin.write_all(&bytes)?;
        }

        let output = if let Some(secs) = timeout_seconds {
            wait_with_timeout(&mut child, Duration::from_secs(secs))?
        } else {
            child
                .wait_with_output()
                .with_context(|| "waiting for python")?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "python script failed: {}\n{}",
                script.display(),
                stderr
            ));
        }

        if self.cfg.debug.keep_python_stderr && !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("python stderr {}: {}", script.display(), stderr.trim());
        }

        let out: O = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("parsing python JSON output: {}", script.display()))?;
        Ok(out)
    }
}

impl Engine for PythonEngine {
    fn doctor(&self) -> Result<DocDiag> {
        let script = self.script("docling_runner.py");
        self.run_json::<serde_json::Value, DocDiag>(
            &script,
            &serde_json::json!({"cmd":"doctor"}),
            Some(30),
            &[],
        )
    }

    fn probe_pdf(&self, input: &Path, sample_pages: u32) -> Result<ProbeOut> {
        let script = self.script("pdf_probe.py");
        let req = serde_json::json!({
            "input_pdf": input,
            "sample_pages": sample_pages,
        });
        let out: ProbeOut = self.run_json(&script, &req, Some(120), &[])?;
        if let Some(err) = out.error.as_deref() {
            return Err(anyhow!("pdf_probe error: {err}"));
        }
        Ok(out)
    }

    fn split_pdf(
        &self,
        input: &Path,
        out_dir: &Path,
        ranges: &[crate::chunk_plan::PageRange],
    ) -> Result<Vec<SplitChunk>> {
        let script = self.script("pdf_split.py");
        let req = serde_json::json!({
            "input_pdf": input,
            "out_dir": out_dir,
            "chunks": ranges,
        });
        let out: SplitOut = self.run_json(&script, &req, Some(300), &[])?;
        if !out.ok {
            let msg = out
                .error
                .unwrap_or_else(|| "pdf_split failed".to_string());
            return Err(anyhow!(msg));
        }
        Ok(out.outputs)
    }

    fn convert_docling(&self, req: &ConvertIn) -> Result<ConvertOut> {
        let script = self.script("docling_runner.py");
        let timeout = if self.cfg.docling.chunk_timeout_seconds > 0 {
            Some(self.cfg.docling.chunk_timeout_seconds)
        } else {
            None
        };
        let out: ConvertOut = self.run_json(
            &script,
            &serde_json::json!({"cmd":"convert","req":req, "cfg": &self.cfg}),
            timeout,
            &[],
        )?;
        if !out.ok {
            warn!("docling convert returned ok=false for chunk {}", req.chunk_index);
        }
        Ok(out)
    }

    fn convert_native_text(&self, req: &ConvertIn) -> Result<ConvertOut> {
        let script = self.script("pdf_text.py");
        let timeout = if self.cfg.docling.chunk_timeout_seconds > 0 {
            Some(self.cfg.docling.chunk_timeout_seconds)
        } else {
            None
        };
        let out: ConvertOut = self.run_json(
            &script,
            &serde_json::json!({"cmd":"convert","req":req, "cfg": &self.cfg}),
            timeout,
            &[],
        )?;
        if !out.ok {
            warn!("native text convert returned ok=false for chunk {}", req.chunk_index);
        }
        Ok(out)
    }
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<Output> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().with_context(|| "try_wait")? {
            let output = collect_output(child, status)?;
            return Ok(output);
        }

        if start.elapsed() > timeout {
            warn!("python process timed out after {:?}", timeout);
            let _ = child.kill();
            let status = child.wait().with_context(|| "wait after kill")?;
            let output = collect_output(child, status)?;
            return Err(anyhow!(
                "python process exceeded timeout ({:?}); stderr: {}",
                timeout,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

fn collect_output(child: &mut Child, status: std::process::ExitStatus) -> Result<Output> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout)
            .with_context(|| "read stdout")?;
    }

    if let Some(mut err) = child.stderr.take() {
        err.read_to_end(&mut stderr)
            .with_context(|| "read stderr")?;
    }

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}
