#!/bin/sh
set -eu

decode_base64() {
  printf '%s' "$1" | base64 --decode 2>/dev/null || printf '%s' "$1" | base64 -D
}

if [ -z "${DART_DEFINES:-}" ]; then
  echo "error: Missing DART_DEFINES. Pass GOOGLE_OAUTH_CLIENT_ID with --dart-define or --dart-define-from-file." >&2
  exit 1
fi

google_oauth_client_id=""

OLD_IFS=$IFS
IFS=','
for encoded in $DART_DEFINES; do
  decoded="$(decode_base64 "$encoded")"
  case "$decoded" in
    GOOGLE_OAUTH_CLIENT_ID=*)
      google_oauth_client_id="${decoded#GOOGLE_OAUTH_CLIENT_ID=}"
      ;;
  esac
done
IFS=$OLD_IFS

if [ -z "$google_oauth_client_id" ]; then
  echo "error: Missing GOOGLE_OAUTH_CLIENT_ID. Pass it with --dart-define or --dart-define-from-file." >&2
  exit 1
fi

client_id_suffix=".apps.googleusercontent.com"
case "$google_oauth_client_id" in
  *"$client_id_suffix")
    client_id_prefix="${google_oauth_client_id%"$client_id_suffix"}"
    ;;
  *)
    echo "error: GOOGLE_OAUTH_CLIENT_ID must end with $client_id_suffix." >&2
    exit 1
    ;;
esac

redirect_scheme="com.googleusercontent.apps.$client_id_prefix"
plist_path="${TARGET_BUILD_DIR}/${INFOPLIST_PATH}"

/usr/libexec/PlistBuddy -c "Set :CFBundleURLTypes:0:CFBundleURLSchemes:0 $redirect_scheme" "$plist_path"

echo "Configured Google OAuth redirect scheme: $redirect_scheme"
