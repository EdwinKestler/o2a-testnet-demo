# Isolated demo development environment

This environment mirrors the O2A specification repository's pinned toolchain
while using a distinct Compose project, network, and volumes. It runs Bitcoin
Core 31.1 and electrs 0.12.0 on an internal regtest network and does not publish
host ports.

```bash
export DOCKER_CONTEXT=default
./dev/check-host.sh
docker compose --file dev/compose.yaml config --quiet
docker compose --file dev/compose.yaml --profile tools build toolchain
docker compose --file dev/compose.yaml --profile tools run --rm toolchain \
  bash -lc 'rustc --version && cargo --version && cargo audit --version && cargo deny --version'
```

Baseline workspace gates:

```bash
docker compose --file dev/compose.yaml --profile tools run --rm toolchain \
  bash -lc '
    set -euo pipefail
    cargo check --workspace --locked
    cargo audit --ignore RUSTSEC-2024-0436
    bash dev/check-dependency-policy.sh
  '
```

`dev/check-dependency-policy.sh` runs `cargo deny check advisories bans sources`
as the blocking gate, then runs `cargo deny check licenses` and records that
exit code without failing the script. License findings are an assessment under
decision D14. The advisory exception is the dated maintainer decision recorded in
`DEMO-GATE.md`; it does not generalize to other unmaintained dependencies.
The 0.11.1 spike uses `spikes/rgb-0.11.1/check-dependency-policy.sh` the same way.

`o2a-demo-core` conformance reads the sibling specification with `git show`.
Inside the toolchain image the repository is `/workspace`, so that relative
path is `/o2a-protocol`. Mount the sibling read-only at that path. Run this
from the demo repository root:

```bash
export DOCKER_CONTEXT=default
docker compose -p o2a-testnet-demo --file dev/compose.yaml --profile tools run --rm \
  -v "$(pwd)/../o2a-protocol:/o2a-protocol:ro" \
  toolchain bash -lc 'cargo test -p o2a-demo-core'
```

`dev/compose.yaml` does not carry this mount. Use the same `-v` when a
workspace gate runs `cargo test`, or the conformance tests fail closed.

Start the local evidence network:

```bash
docker compose --file dev/compose.yaml --profile rgb up --detach --wait bitcoin electrs
```

Smoke project: `docker compose -p o2a-seal-smoke --file dev/compose.yaml --file dev/compose.smoke.yaml --profile rgb up --detach --wait bitcoin electrs`
Smoke evidence (2026-09-25) reproduces only at commit bce2b58; the crate on main is a compiled code reference.
2026-09-25: that override also passes `-rpcallowip=172.30.32.0/24` on the bitcoind command; `dev/bitcoin.conf` stays on `172.30.30.0/24`.

The toolchain container can reach `electrs:50001` on the internal network.
Regtest wallets, keys, identities, and values are disposable. Never mount real
wallets or credentials. Do not delete Compose volumes unless the maintainer
explicitly requests destructive cleanup.
