#!/usr/bin/env bash
# Puts hawk/compose.yaml on hawk in /opt/vmate and starts it; alchemy.run.ts runs this with the
# images and secrets in the environment. Secrets reach hawk on ssh's stdin, never a command line.
set -euo pipefail
cd "$(dirname "$0")"
: "${HAWK:?}" "${ENGINE_IMAGE:?}" "${GATEWAY_IMAGE:?}" "${VERIFIER_IMAGE:?}" "${CONVEX_URL:?}" "${INTERNAL_TOKEN:?}" \
  "${GATEWAY_TOKEN:?}" "${VAULT_KEY:?}" "${VERIFIER_TOKEN:?}" "${GHCR_USERNAME:?}" "${GHCR_TOKEN:?}"
remote() { ssh -o BatchMode=yes "$HAWK" "$@"; }

remote 'mkdir -p /opt/vmate'
scp -q -o BatchMode=yes compose.yaml "$HAWK:/opt/vmate/compose.yaml"
printf 'ENGINE_IMAGE=%s\nGATEWAY_IMAGE=%s\nVERIFIER_IMAGE=%s\nGATEWAY_TOKEN=%s\n' "$ENGINE_IMAGE" "$GATEWAY_IMAGE" "$VERIFIER_IMAGE" "$GATEWAY_TOKEN" |
  remote 'umask 077 && cat > /opt/vmate/.env'
printf 'CONVEX_URL=%s\nINTERNAL_TOKEN=%s\nGATEWAY_TOKEN=%s\nVAULT_KEY=%s\n' "$CONVEX_URL" "$INTERNAL_TOKEN" "$GATEWAY_TOKEN" "$VAULT_KEY" |
  remote 'umask 077 && cat > /opt/vmate/engine.env'
# The verifier gets only its own token: no VAULT_KEY, INTERNAL_TOKEN or GATEWAY_TOKEN.
printf 'VERIFIER_TOKEN=%s\n' "$VERIFIER_TOKEN" |
  remote 'umask 077 && cat > /opt/vmate/verifier.env'

# The token comes on stdin and the login is dropped after the pull, so hawk keeps no
# registry credential and the images can stay private.
printf '%s\n' "$GHCR_TOKEN" | remote "set -e
cd /opt/vmate
docker login ghcr.io -u '$GHCR_USERNAME' --password-stdin >/dev/null
trap 'docker logout ghcr.io >/dev/null 2>&1' EXIT
"'
# Convex keeps the INSTANCE_SECRET its first compose made (/opt/vmate-convex, now retired).
if [ ! -s convex.env ]; then
  test -s /opt/vmate-convex/.env
  (umask 077 && cp /opt/vmate-convex/.env convex.env)
fi
# Pulled before anything stops: a missing image or a refused pull leaves the running setup alone.
docker compose pull -q
# Before this compose, Convex ran from /opt/vmate-convex and the gateway and engine were loose
# containers; they hold the same ports, so they go first. Their data stays (volume, key folder).
if [ -f /opt/vmate-convex/compose.yaml ]; then
  (cd /opt/vmate-convex && docker compose down)
  mv /opt/vmate-convex/compose.yaml /opt/vmate-convex/compose.yaml.retired
fi
for c in vitapmate-gateway vitapmate-gateway-v3 vmate-engine; do
  if docker inspect "$c" >/dev/null 2>&1; then docker rm -f "$c" >/dev/null; fi
done
docker compose up -d --remove-orphans --wait --wait-timeout 120'
