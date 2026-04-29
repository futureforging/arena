#!/usr/bin/env bash
# Start one Verity agent against the production Arena.
#
# Usage:
#   ./scripts/play.sh missionary <invite>
#   ./scripts/play.sh friend     <invite>
#
# Conventions baked in:
#   missionary -> arena_signing_key_1.hex, signer port 8090, role=first
#   friend     -> arena_signing_key_2.hex, signer port 8091, role=second
#
# Prerequisites (started separately, ONCE):
#   just run-runtime   # the Omnia HTTP listener on 127.0.0.1:8080
#
# Both signing keys must exist at the workspace root. Generate them once with:
#   openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\n' > arena_signing_key_N.hex

set -euo pipefail

# Resolve to the workspace root (parent of this script's directory) so relative
# paths (key files, cargo) work regardless of where the script is invoked from.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

NAME="${1:?missing agent name (missionary|friend)}"
INVITE="${2:?missing invite code (e.g. inv_...)}"

case "$NAME" in
  missionary) KEY_NUM=1; PORT=8090; ROLE=first  ;;
  friend)     KEY_NUM=2; PORT=8091; ROLE=second ;;
  *)
    echo "agent name must be 'missionary' or 'friend' (got '$NAME')" >&2
    exit 1
    ;;
esac

KEY_FILE="arena_signing_key_${KEY_NUM}.hex"
if [ ! -f "$KEY_FILE" ]; then
  echo "missing $KEY_FILE in $(pwd)" >&2
  echo "generate it with:" >&2
  echo "  openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\n' > $KEY_FILE" >&2
  exit 1
fi

# --- Signer lifecycle ---
echo "[$NAME] starting signer on 127.0.0.1:$PORT ($KEY_FILE)"
VERITY_SIGNER_ADDR="127.0.0.1:$PORT" \
  VERITY_ARENA_SIGNING_KEY_FILE="$KEY_FILE" \
  cargo run --quiet -p verity-runtime --bin verity-signer &
SIGNER_PID=$!
trap 'echo "[$NAME] stopping signer (pid $SIGNER_PID)"; kill "$SIGNER_PID" 2>/dev/null || true' EXIT INT TERM

# Wait for /pubkey to respond (signer is ready).
echo "[$NAME] waiting for signer to bind..."
for i in $(seq 1 60); do
  if curl -sS --max-time 1 "http://127.0.0.1:$PORT/pubkey" > /dev/null 2>&1; then
    echo "[$NAME] signer ready"
    break
  fi
  sleep 0.5
  if [ "$i" = "60" ]; then
    echo "[$NAME] signer did not respond on /pubkey after 30s — aborting" >&2
    exit 1
  fi
done

# --- /play call ---
echo "[$NAME] starting play (role=$ROLE, invite=$INVITE)"
curl -sS -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  --max-time 600 \
  -d "$(cat <<EOF
{
  "game": "psi",
  "arena_url": "https://arena-engine.nicolaos.org",
  "invite": "$INVITE",
  "signer_url": "http://127.0.0.1:$PORT",
  "role": "$ROLE",
  "username": "$NAME"
}
EOF
)"
echo
echo "[$NAME] /play returned"
