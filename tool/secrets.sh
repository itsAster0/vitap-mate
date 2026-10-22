#!/usr/bin/env bash
# Writes the project's secret files from the "VITAP Mate" 1Password
# Environment. Locally op signs in through the 1Password app; in CI it uses the
# service account in OP_SERVICE_ACCOUNT_TOKEN. Every file is written mode 0600
# and is gitignored.
#
#   tool/secrets.sh                     all of them
#   tool/secrets.sh env signing         only these
#
#   env       .env.json                         Dart defines for every build
#   signing   android/keystore.properties and   release signing
#             android/upload-keystore.jks
#   play      android/fastlane/key.json         Play Console uploads (fastlane)
#   firebase  android/app/google-services.json  native Firebase config
#
# Files are stored in the Environment base64-encoded (*_BASE64 variables).
set -euo pipefail
cd "$(dirname "$0")/.."

environment=u7uzop6464ilrrd3b6wgblrxou

# Re-run inside op run so the Environment's variables are in our env.
# --no-masking: op run otherwise masks the values even in redirected output.
if [ -z "${VITAP_SECRETS_LOADED:-}" ]; then
  VITAP_SECRETS_LOADED=1 exec op run --no-masking \
    --environment "$environment" -- "$0" "$@"
fi

targets=("$@")
[ ${#targets[@]} -gt 0 ] || targets=(env signing play firebase)

umask 077

# put PATH COMMAND...: COMMAND's output replaces PATH (and its mode) only if
# COMMAND succeeds. A missing variable fails while the arguments expand, before
# anything is written, so an existing file is never truncated.
put() {
  local path=$1
  shift
  "$@" >"$path.tmp" || {
    rm -f "$path.tmp"
    return 1
  }
  mv -f "$path.tmp" "$path"
}

decode() {
  base64 --decode <<<"$1"
}

for target in "${targets[@]}"; do
  case $target in
    env)
      # Fail on a missing variable rather than writing null.
      put .env.json jq -en '$ENV | {GOOGLE_OAUTH_CLIENT_ID,
        FIREBASE_ANDROID_API_KEY, FIREBASE_ANDROID_APP_ID,
        FIREBASE_MESSAGING_SENDER_ID, FIREBASE_PROJECT_ID,
        FIREBASE_STORAGE_BUCKET, VTOP_EXTENSION_URL, VTOP_SERVER_URL}
        | if any(.[]; . == null) then error("missing variable") end'
      ;;
    signing)
      put android/keystore.properties printf \
        'password=%s\nkeyAlias=%s\nstoreFile=../upload-keystore.jks\n' \
        "$ANDROID_KEYSTORE_PASSWORD" "$ANDROID_KEY_ALIAS"
      put android/upload-keystore.jks decode "$ANDROID_UPLOAD_KEYSTORE_BASE64"
      ;;
    play)
      put android/fastlane/key.json decode "$PLAY_SERVICE_ACCOUNT_JSON_BASE64"
      ;;
    firebase)
      put android/app/google-services.json \
        decode "$GOOGLE_SERVICES_JSON_BASE64"
      ;;
    *)
      echo "Unknown secret: $target (env, signing, play, firebase)" >&2
      exit 2
      ;;
  esac
  echo "Wrote $target"
done
