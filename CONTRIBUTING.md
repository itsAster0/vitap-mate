# Contributing

Thanks for your interest in contributing.

New features are welcomed.

## What Contributions Are Accepted

- New features
- Security fixes
- Bug fixes
- Documentation improvements

## Development Setup

1. Install [mise](https://mise.jdx.dev), Rust (stable) with `cargo-ndk`, and the Android SDK and NDK.
2. Run `mise install`.
3. Run `mise run rust-android`.
4. Run `./kotlin build`.
5. Run `./kotlin test -m core`.

See the Development section of the [README](README.md) for running the app and release builds.

## Pull Request Guidelines

1. Keep changes focused and minimal.
2. Include clear reproduction steps for bugs.
3. Use concise commit messages that explain intent.

## Code Style

- Follow existing project style and lints.
- Prefer small, reviewable PRs.
