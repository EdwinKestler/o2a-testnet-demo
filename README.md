# O2A Testnet Demo

This is the disposable O2A demonstration workspace permitted by the
2026-09-24 demo-lineage entry in the O2A protocol specification. It is a
separate repository and is not part of the normative specification.

The workspace contains:

- `o2a-demo-core`: deterministic O2A encoding, signing, and verification;
- `o2a-demo-rgb`: the RGB 0.12 RC3 adapter; and
- `o2a-demo-cli`: orchestration commands for identity creation, controller
  rotation, attestation issuance, and package verification.

Only identity creation and controller rotation are implemented in the first
execution milestone. Attestations are command-surface placeholders for the
next milestone. There is no UI.

Regtest is the current development and evidence network. The default public
Bitcoin signet is the declared live demonstration target. Testnet3 is not
wired, and mainnet is out of scope.

Read [DEMO-GATE.md](DEMO-GATE.md) before running or changing the demo.
Every identity created here is disposable and permanently unsuitable for
mainnet or production use.

## Isolated development environment

The host needs Git, Docker Engine, and Docker Compose v2. Rust, Bitcoin Core,
electrs, and audit tools run in containers:

```bash
export DOCKER_CONTEXT=default
./dev/check-host.sh
docker compose --file dev/compose.yaml --profile tools build toolchain
docker compose --file dev/compose.yaml --profile tools run --rm toolchain \
  bash -lc 'cargo check --workspace --locked'
```

See [dev/README.md](dev/README.md) for the complete workflow.

## License

O2A-authored code and documentation are available under `MIT OR Apache-2.0`.
Third-party dependencies retain their own licenses.
