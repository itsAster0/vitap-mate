fastlane documentation
----

# Installation

Make sure you have the latest version of the Xcode command line tools installed:

```sh
xcode-select --install
```

For _fastlane_ installation instructions, see [Installing _fastlane_](https://docs.fastlane.tools/#installing-fastlane)

# Available Actions

## Android

### android upload_internal

```sh
[bundle exec] fastlane android upload_internal
```

Build (`mise run bundle`, skip with build:false) and upload the AAB to internal, with changelogs/<versionCode>.txt

### android promote_testing

```sh
[bundle exec] fastlane android promote_testing
```

Promote the latest internal build to the closed (friends), alpha and beta tracks

### android upload_testing

```sh
[bundle exec] fastlane android upload_testing
```

upload_internal, then promote_testing: one build on internal, friends, alpha and beta (never production)

### android promote_production

```sh
[bundle exec] fastlane android promote_production
```

Promote the beta build to production as a staged rollout (rollout:0.2 by default)

### android widen_rollout

```sh
[bundle exec] fastlane android widen_rollout
```

Widen production's staged rollout (rollout:0.5), or rollout:1 for everyone

### android status

```sh
[bundle exec] fastlane android status
```

Show the release on each track (version and code); uploads nothing

----

This README.md is auto-generated and will be re-generated every time [_fastlane_](https://fastlane.tools) is run.

More information about _fastlane_ can be found on [fastlane.tools](https://fastlane.tools).

The documentation of _fastlane_ can be found on [docs.fastlane.tools](https://docs.fastlane.tools).
