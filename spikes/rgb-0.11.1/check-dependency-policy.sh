#!/usr/bin/env bash
# RGB 0.11.1 spike gate. Licenses are report-only under D14.
# MITNFA is assessed in the o2a-protocol license register; it is not allowlisted.
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
manifest="$root/Cargo.toml"

cargo deny --manifest-path "$manifest" check advisories bans sources

license_status=0
cargo deny --manifest-path "$manifest" check licenses || license_status=$?
printf 'cargo deny licenses report-only: exit %s\n' "$license_status"
