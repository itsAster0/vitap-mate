---
name: add-vtop-page
description: Add a new VTOP read page end to end, from the parser in vtop-proto to a screen that draws it with rememberPage. Use when a feature needs data from a VTOP page the app doesn't fetch yet.
---

# Add a VTOP page

A read page needs code in vtop-proto, vtop-ffi and the feature, and nothing in
vtop-core or vtop-bridge: both run every spec in `spec::each`. Writes and
downloads still keep a method in `rust/vtop-core/src/client/`.

## Steps

1. **Parser.** `rust/vtop-proto/src/parser/<page>.rs` (add it to `parser/mod.rs`),
   returning a type from `vtop-proto`'s types. Parsers never panic: skip rows
   that don't fit and read missing cells as empty.
2. **Parser test.** Save a synthetic or redacted page as
   `rust/vtop-proto/tests/fixtures/<page>.html` (never a real student's page),
   add a case in `tests/parsers.rs`, then write its snapshot with
   `UPDATE_SNAPSHOTS=1 cargo test -p vtop-proto --test parsers`. Read the new
   `<page>.expected.json`; never edit it by hand.
3. **Machine.** The request(s) in `rust/vtop-proto/src/pages.rs`, next to the
   others (`pages::mentor_details` is a small one).
4. **Spec.** A `pub const X: PageSpec<T>` in `rust/vtop-proto/src/spec.rs`
   (name, args, cache file, build) and `visit.page(&X)` at the **end** of
   `spec::each`. Run `cargo test -p vtop-proto specs_keep_their_contract`: it
   prints the line to append to `rust/vtop-proto/tests/spec_contract.txt`.
   Append it in the same change. Never rename or reorder an existing spec's
   name, args or file: old apps and the deployed engine use them.
5. **FFI.** In the feature's file `rust/vtop-ffi/src/<feature>.rs` (new file:
   `mod` it in `lib.rs`), its own `#[uniffi::export] impl Core` block with
   `cached_x(flow)` and `fetch_x(flow, ...)`, flow id first:
   - `fetch_x` calls `Core::fetch(&spec::X, ...)` (cached under the picked
     semester), or `Core::page` for a page that isn't cached.
   - `cached_x` reads it back with `read_page`.
   `profile.rs` is a short example. Add a test next to the module or in
   `rust/vtop-ffi/src/tests.rs`.
6. **Bindings.** `mise run rust-android` (or `mise run rust-android-dev` for a
   quick arm64 run). Commit the regenerated `core/src/vitapmate/core/vtop_ffi.kt`;
   CI fails when it is out of date. Never edit it by hand.
7. **Screen.** In the feature, off the main thread and once per screen state:
   `rememberPage("<id>", { c, f -> c.cachedX(f) }, { c, f -> c.fetchX(f) }, { it.updatedAt }) { ... }`
   drawn with `PageScaffold` (`core/src/vitapmate/data/Page.kt`).

## Touch every surface

- Routes: try it on Direct, Proxy, Server and Udp (Settings → Connection).
- Cached and fresh: open the screen with no cache, then again offline.
- vtop-mcp: add a `get_x` tool in `rust/vtop-mcp/src/tools.rs` if agents should read it.
- Demo: add the page to `edge/guides/demo.json` under `pages`,
  keyed `name@semester?arg=value` like the others, so the demo account shows it.
- updates.json: give the feature `"since": <versionCode>` in
  `edge/guides/updates.json`.
- The edge needs nothing for a read page unless the spec's shape is new to it;
  check `edge/API.md`.

## Check

```sh
cd rust && cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
./kotlin test
mise run check-changed
```

Then run it on the emulator (see the `test-on-emulator` skill).
