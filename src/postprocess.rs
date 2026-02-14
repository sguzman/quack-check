use crate::config::Config;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

pub fn merge_markdown(cfg: &Config, parts: Vec<String>) -> Result<String> {
    let mut merged = parts.join("\n\n---\n\n");

    if cfg.postprocess.normalize_newlines {
        merged = merged.replace("\r\n", "\n");
    }

    if cfg.postprocess.normalize_unicode {
        merged = merged.nfkc().collect::<String>();
    }

    merged = sanitize_control_chars(&merged, &cfg.postprocess.control_chars_to_sanitize);

    if cfg.postprocess.trim_trailing_whitespace {
        merged = merged
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
    }

    if cfg.postprocess.remove_repeated_lines {
        merged = remove_repeated_lines(cfg, &merged);
    }

    if cfg.postprocess.remove_by_regex {
        merged = remove_by_regex(cfg, &merged)?;
    }

    Ok(merged)
}

fn sanitize_control_chars(s: &str, codes: &[u8]) -> String {
    if codes.is_empty() {
        return s.to_string();
    }

    let mut mask = [false; 128];
    for &code in codes {
        if (code as usize) < mask.len() {
            mask[code as usize] = true;
        }
    }

    s.chars()
        .filter(|&ch| {
            // Preserve structural whitespace controls to avoid flattening documents.
            if ch == '\n' || ch == '\r' || ch == '\t' {
                return true;
            }
            let cp = ch as u32;
            if cp < 128 {
                !mask[cp as usize]
            } else {
                true
            }
        })
        .collect()
}

fn remove_repeated_lines(cfg: &Config, s: &str) -> String {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    let lines: Vec<&str> = s.lines().collect();

    for &l in &lines {
        let l2 = l.trim();
        if l2.is_empty() {
            continue;
        }
        if l2.len() > cfg.postprocess.repeated_line_max_length as usize {
            continue;
        }
        *counts.entry(l2).or_insert(0) += 1;
    }

    let min = cfg.postprocess.repeated_line_min_occurrences;
    let mut out = Vec::with_capacity(lines.len());
    for &l in &lines {
        let l2 = l.trim();
        let keep = if l2.is_empty() {
            true
        } else {
            counts.get(l2).copied().unwrap_or(0) < min
        };
        if keep {
            out.push(l);
        }
    }
    out.join("\n")
}

fn remove_by_regex(cfg: &Config, s: &str) -> Result<String> {
    let regs: Vec<Regex> = cfg
        .postprocess
        .regex
        .patterns
        .iter()
        .map(|p| Regex::new(p))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut out = Vec::new();
    for line in s.lines() {
        let mut matched = false;
        for r in &regs {
            if r.is_match(line.trim()) {
                matched = true;
                break;
            }
        }
        if !matched {
            out.push(line);
        }
    }
    Ok(out.join("\n"))
}

pub fn markdown_to_text(_cfg: &Config, md: &str) -> Result<String> {
    let mut s = md.replace("**", "");
    s = s.replace("# ", "");
    s = s.replace("## ", "");
    s = s.replace("### ", "");
    Ok(s)
}
