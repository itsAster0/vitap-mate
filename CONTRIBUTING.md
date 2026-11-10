# Contributing

Pull requests for new features, bug fixes, security fixes and docs are welcome. Report a vulnerability privately as [SECURITY.md](SECURITY.md) describes, not in an issue.

## Setup

1. Install [mise](https://mise.jdx.dev), Rust (stable) with `cargo-ndk`, and the Android SDK and NDK.
2. Run `mise install`.
3. Run `mise run rust-android`.
4. Run `./kotlin build`.
5. Run `./kotlin test -m core`.

The Development section of the [README](README.md) shows how to run the app on an emulator or a phone.

## Where code goes

Each screen is its own module in `features/<name>/` and depends only on `design`. A feature never imports another one; shared UI goes in `design/`, shared data in `core/`. A new VTOP page needs a parser in `rust/vtop-proto`, a call in `rust/vtop-core` and an export in `rust/vtop-ffi`. `AGENTS.md` has the full table, and `rust/ARCHITECTURE.md` explains the Rust crates and the server.

## Pull requests

- Keep one change per pull request, with a small diff.
- Keep the look of the app the same unless the change is about the look. Use Material 3 Expressive components and theme colours.
- For a bug, write the steps that show it.
- Test on an emulator or a phone, not only with a build, and say what you tested.
- Run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` in `rust/` when you change Rust.
- Never log cookies, passwords, OTPs, CSRF tokens or keys.
- Write commit messages as Conventional Commits (`feat:`, `fix:`, `docs:`), with a subject under 72 characters.
