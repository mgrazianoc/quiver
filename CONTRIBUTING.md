# Contributing to quiver

Thanks for your interest in contributing! This document covers the process and guidelines.

## Getting Started

```bash
git clone https://github.com/mgrazianoc/quiver.git
cd quiver
cargo build
cargo run
```

**Requirements:** Rust 1.75+ (stable)

## Development Workflow

1. Fork the repository
2. Create a feature branch from `main`: `git checkout -b feat/your-feature`
3. Make your changes
4. Ensure CI passes locally:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features
   cargo check --all-features
   cargo test --all-features
   ```

5. Commit with clear, descriptive messages (see below)
6. Open a Pull Request against `main`

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```ascii
feat: add column sorting in results viewer
fix: correct cursor position after tab deletion
refactor: extract theme loading into separate module
docs: update keybinding table in README
ci: add clippy lint step
```

## Code Style

- Run `cargo fmt` before committing — CI enforces this
- Fix all `cargo clippy` warnings — CI treats warnings as errors (`RUSTFLAGS="-Dwarnings"`)
- Keep functions focused and small
- Prefer descriptive names over comments
- Add comments only when the *why* isn't obvious from the code

## Architecture Overview

The codebase follows a layered structure:

- **`app.rs`** — Application state and event dispatch (the "model")
- **`event.rs`** — Terminal event reader
- **`ui/`** — All rendering logic (the "view") — panes, tabs, statusbar, command palette
- **`core/`** — Flight SQL client and Arrow data layer (future)
- **`config/`** — Configuration loading
- **`theme/`** — Theme definitions
- **`keybindings/`** — Key mode detection and mapping

Rendering is stateless: `ui::render()` reads from `App` and draws. State changes happen in `App::handle_event()`.

## What to Work On

Check the [issues](https://github.com/mgrazianoc/quiver/issues) for tasks labeled:

- `good first issue` — self-contained, well-defined tasks
- `help wanted` — larger features open for contribution
- `bug` — confirmed bugs needing fixes

The roadmap in [README.md](README.md) outlines the milestone plan.

## Pull Request Guidelines

- One logical change per PR
- Update `CHANGELOG.md` under `[Unreleased]` for user-facing changes
- Add tests for new functionality when applicable
- Keep PRs small and reviewable — split large features into incremental PRs

## Reporting Bugs

Use the [bug report template](https://github.com/mgrazianoc/quiver/issues/new?template=bug_report.md). Include:

- Terminal emulator and OS
- Steps to reproduce
- Expected vs. actual behavior

## License

By contributing, you agree that your contributions will be licensed under the project's dual MIT OR Apache-2.0 license.
