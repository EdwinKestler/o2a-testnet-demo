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
    cargo deny check
  '
```

The advisory exception is the dated maintainer decision recorded in
`DEMO-GATE.md`; it does not generalize to other unmaintained dependencies.

Start the local evidence network:

```bash
docker compose --file dev/compose.yaml --profile rgb up --detach --wait bitcoin electrs
```

Smoke project: `docker compose -p o2a-seal-smoke --file dev/compose.yaml --file dev/compose.smoke.yaml --profile rgb up --detach --wait bitcoin electrs`

The toolchain container can reach `electrs:50001` on the internal network.
Regtest wallets, keys, identities, and values are disposable. Never mount real
wallets or credentials. Do not delete Compose volumes unless the maintainer
explicitly requests destructive cleanup.
