# quack-check

quack-check is a deterministic PDF transcript orchestrator built around Docling and policy-driven extraction.

## Intent

Handle the messy reality of PDFs by classifying input quality, selecting an extraction policy, chunking safely when needed, and producing stable merged transcripts.

## Ambition

The project is aiming to be the orchestration layer around PDF transcript generation rather than another generic text extractor, with deterministic policy and auditability as core design goals.

## Current Status

The codebase already includes CLI/config/pipeline/policy/reporting modules, tests, and example config. It looks like a serious pipeline tool rather than a prototype.

## Core Capabilities Or Focus Areas

- PDF quality probing and policy selection.
- Chunk planning for large or difficult PDFs.
- Docling/native extraction orchestration.
- Transcript post-processing and report generation.
- Deterministic pipeline behavior with explicit policy modules.

## Project Layout

- `res/`: bundled resources used by the application.
- `scripts/`: helper scripts for development, validation, or release workflows.
- `src/`: Rust source for the main crate or application entrypoint.
- `tests/`: automated tests, fixtures, or parity scenarios.
- `Cargo.toml`: crate or workspace manifest and the first place to check for package structure.

## Setup And Requirements

- Rust toolchain.
- PDF inputs.
- Any external extraction/runtime dependencies required by the configured Docling workflow.

## Build / Run / Test Commands

```bash
cargo build
cargo test
cargo run -- --help
```

## Notes, Limitations, Or Known Gaps

- This tool is about orchestration and policy, not just raw extraction.
- Determinism is part of the product contract here, so policy changes should be treated carefully.

## Next Steps Or Roadmap Hints

- Keep policy decisions explainable and reviewable as more PDF classes are added.
- Add more fixtures for borderline or partially broken PDFs to prevent silent regressions.
