#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
compose_file="${repo_root}/dev/compose.yaml"
probe_image='debian:bookworm-slim@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251'

test "$(uname -s)" = Linux
test "$(uname -m)" = x86_64
command -v docker >/dev/null
docker version
docker compose version
docker info >/dev/null
docker compose --file "${compose_file}" config --quiet

if ! docker run --rm \
  --mount "type=bind,src=${repo_root},dst=/workspace,readonly" \
  "${probe_image}" test -f /workspace/README.md; then
  printf '%s\n' \
    'ERROR: the active Docker context cannot bind-mount this repository.' \
    'If the native Linux engine is installed, retry in this shell with:' \
    '  export DOCKER_CONTEXT=default' >&2
  exit 1
fi

printf 'Docker context: %s\n' "$(docker context show)"
printf '%s\n' 'O2A testnet demo host prerequisites: PASS'
