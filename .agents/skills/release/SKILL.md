---
name: release
description: Prepare an app release for Google Play (version, changelog, updates.json, update priority, tag). Use only when the user asks for a release; every step that uploads or deploys needs their explicit OK.
---

# Release

The steps live in the **Release** section of `AGENTS.md`. Read it first and
follow it; this skill only adds the order of work and the stops. If the two
disagree, `AGENTS.md` wins: fix this file.

Nothing here runs without the user asking for a release. Each step marked
**ask** uploads or deploys: say what it will do and wait for an OK for that step.

## Order

1. Read `AGENTS.md` → Release, and the last release's commit
   (`jj log -r 'description(glob:"chore(release)*")' -n 1`, or the last `v*` tag).
2. `mise run notes` drafts the notes from the commits since then and stubs this
   versionCode's Play changelog and its `updates.json` entry. Edit the drafts.
3. In one change:
   - `versionCode` and `versionName` in `app/module.yaml` (the code scheme is in
     `AGENTS.md`; Play refuses a code it has seen).
   - `fastlane/metadata/android/en-GB/changelogs/<versionCode>.txt`, at most
     500 characters (`wc -m` it).
   - The release, newest first, in `edge/guides/updates.json`, with
     `"since": <versionCode>` on the features it adds.
   - A line in `UPDATE_PRIORITIES` in `fastlane/Fastfile` if the priority isn't
     the default (2).
4. Check: `mise run check`, then a release build on the emulator
   (`mise run release`, `adb install -r build/vitapmate-release.apk`).
5. Commit only when asked (`chore(release): <versionName>`), with jj.
6. **ask** `bun run deploy` in `edge/` (updates.json goes out with the edge).
7. **ask** Push the tag `v<versionName>`. `android-release.yml` checks it
   matches `versionName`, builds the signed AAB and uploads it to the internal track.
8. **ask** Promotion: the Play Console, or
   `bundle exec fastlane android promote_all_testing`. Production stays manual.

## Don't

- Run `fastlane android deploy_internal` or push a tag to "test" something.
- Reuse a versionCode, or change one that was uploaded.
- Read or print `keystore/`, `app/keystore.properties` or `fastlane/key.json`.
