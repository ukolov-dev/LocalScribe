# Contributing

Thanks for taking the time to improve LocalScribe.

## Before You Start

LocalScribe is a macOS desktop app built with:

- Tauri 2
- React
- TypeScript
- Rust
- `whisper-rs` / `whisper.cpp`

The project is privacy-first. Changes must preserve the expectation that user media files stay local unless the user explicitly chooses otherwise.

## Development Setup

Install dependencies:

```bash
npm ci
```

Run the app locally:

```bash
npm run tauri dev
```

Run checks before opening a pull request:

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

For faster backend feedback:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## Pull Request Guidelines

- Keep changes focused.
- Include tests when changing Rust logic, timestamp formatting, model validation, export behavior, or diarization behavior.
- Update frontend and backend shared types together.
- Update documentation when changing commands, setup, model behavior, storage paths, or release behavior.
- Do not commit generated build output, downloaded models, local settings, or secrets.

## Code Style

- Prefer the existing structure and naming conventions.
- Keep Tauri command payloads serializable with `serde` and compatible with the TypeScript types in `src/types/localscribe.ts`.
- Keep UI copy clear and short.
- Keep privacy-sensitive behavior explicit.

## Reporting Bugs

When filing a bug, include:

- macOS version and Mac model.
- LocalScribe version or commit.
- Input file type.
- Selected model and language mode.
- Whether Metal/GPU was enabled.
- Reproduction steps.
- Relevant logs or error messages.

Do not attach private audio/video files to public issues.

## Security Issues

Please do not report vulnerabilities in public issues. See `SECURITY.md`.
