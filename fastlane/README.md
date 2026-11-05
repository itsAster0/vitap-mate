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

### android deploy_internal

```sh
[bundle exec] fastlane android deploy_internal
```

Deploy a new version to the Internal track only

### android promote_all

```sh
[bundle exec] fastlane android promote_all
```

Promote latest internal build to closed, open and production

### android deploy_all

```sh
[bundle exec] fastlane android deploy_all
```

Build & upload to internal, then promote through all tracks

### android deploy_actions

```sh
[bundle exec] fastlane android deploy_actions
```

Build & upload for actions

----

This README.md is auto-generated and will be re-generated every time [_fastlane_](https://fastlane.tools) is run.

More information about _fastlane_ can be found on [fastlane.tools](https://fastlane.tools).

The documentation of _fastlane_ can be found on [docs.fastlane.tools](https://docs.fastlane.tools).
