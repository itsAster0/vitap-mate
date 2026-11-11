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

Build (`mise run bundle`, skip with build:false) and upload the AAB to internal, with changelogs/<versionCode>.txt

### android promote_all

```sh
[bundle exec] fastlane android promote_all
```

Promote the latest internal build to the closed (friends), alpha and beta tracks

### android deploy_all

```sh
[bundle exec] fastlane android deploy_all
```

Build, upload to internal, then promote through the testing tracks

----

This README.md is auto-generated and will be re-generated every time [_fastlane_](https://fastlane.tools) is run.

More information about _fastlane_ can be found on [fastlane.tools](https://fastlane.tools).

The documentation of _fastlane_ can be found on [docs.fastlane.tools](https://docs.fastlane.tools).
