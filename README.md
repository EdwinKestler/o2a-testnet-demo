# O2A Testnet Demo

This is the disposable O2A demonstration workspace permitted by the
2026-09-24 demo-lineage entry in the O2A protocol specification. It is a
separate repository and is not part of the normative specification.

The static material in [`site/`](site/) supports the live demo and is not protocol authority.

The workspace contains:

- `o2a-demo-core`: deterministic O2A encoding, signing, and verification.
  It has no RGB dependency and is the default Cargo member.
- `spikes/rgb-0.11.1`: the maintained rgb-protocol 0.11.1 adapter, Opret,
  in its own Cargo workspace. Read [docs/rgb-0.11.1-adapter.md](docs/rgb-0.11.1-adapter.md).
- `o2a-demo-rgb`: the archived RGB 0.12 RC3 adapter. It stays in the
  workspace so that graph still resolves, and it is not a default member.
  Its program note is [docs/rgb-program-demo.md](docs/rgb-program-demo.md).
- `o2a-demo-cli`: orchestration commands for the archived adapter.

The public stage plan follows ADR-0008, ADR-0009, and ADR-0010. The stage
is one genesis plus one `official_name` claim. A claim is a signed object
and spends no Bitcoin. Controller rotation, recovery, and revocation stay
off that stage until the RGB program is final. Those operations exist in
disposable lineage evidence. The operator runbook is
[docs/block0-runbook.md](docs/block0-runbook.md). The commercial script is
[docs/guion-demo.md](docs/guion-demo.md).

Regtest is the development and evidence network. Signet is the rehearsal
network. The network for a later public event is a maintainer decision.
This repository does not perform a mainnet mint. Mainnet is out of scope.

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

## Evidence index

Every bundle under `evidence/` is listed once. Status is `current`,
`superseded`, `archived 0.12`, or `correction`. Line is `0.12` or `0.11.1`.
The `0.12` line is RGB-WG v0.12.0-rc.3. The electrs image tag in the signet
acceptance bundle is a separate fact and is not this line column.

| Bundle | Status | Line | Record |
| --- | --- | --- | --- |
| `evidence/regtest-genesis-rotation-2026-09-24` | superseded | 0.12 | Pre-seal-policy genesis and controller rotation. |
| `evidence/signet-acceptance-2026-09-24` | superseded | 0.12 | Signet stack acceptance from that demo era. |
| `evidence/regtest-seal-tapscript-smoke-2026-09-25` | superseded | 0.12 | Custom tapscript seals on RGB 0.12 RC3. |
| `evidence/regtest-seal-tapscript-smoke-correction-2026-09-25` | correction | 0.12 | Correction of the smoke bundle. That bundle was not edited. |
| `evidence/regtest-seal-policy-lineage-2026-09-26` | superseded | 0.12 | Shared-root seal-policy lineage. |
| `evidence/regtest-seal-policy-lineage-correction-2026-09-28` | correction | 0.12 | Identities 1 and 2 shared entity index 0. The 2026-09-26 bundle was not edited. |
| `evidence/regtest-genesis-bound-lineage-2026-09-28` | archived 0.12 | 0.12 | ADR-0008 EntityID evidence on the archived adapter. Rotation and recovery ran. |
| `evidence/signet-block0-rehearsal-2026-09-28` | superseded | 0.12 | First signet block-0 rehearsal. Gate 3 rejected its record-trust verifier. Authority `b622c983`. |
| `evidence/signet-block0-rehearsal-2-2026-09-28` | superseded | 0.12 | Clean two-directory verifier fix. Superseded by rehearsal 3. |
| `evidence/regtest-rgb011-compat-2026-09-28` | current | 0.11.1 | Compatibility check. The original NO-GO text stays. Manifest file `2b376eed8bb5e5640fe542fbf4b5b4cb7d63315502a4de9f7115b18e16bd2cad`. |
| `evidence/regtest-rgb011-compat-2026-09-28-addendum` | current | 0.11.1 | D14 rerun. Licenses are report-only. `hex_lit 0.1.1` MITNFA is registered, not allowlisted. Manifest file `e9acd79fcd911cd7d01b74ec3ffd01570bf7aa78a554db0a68f3185b2b931367`. |
| `evidence/regtest-rgb011-lineage-2026-09-28` | current | 0.11.1 | Maintained-adapter regtest lineage. H0 108. Manifest file `ba5479df552f8fcea7c27e9b4ffae5ac67d40184f1043ba108b134d6e6020849`. |
| `evidence/signet-block0-rehearsal-3-2026-09-28` | current | 0.11.1 | First 0.11.1 signet rehearsal. Manifest file `2d98449550921243ebaec876b732bb68a3f6fbda2ca765f4b0c7cbb3c91ea91f`. |

These bundles close no Phase 0 gate.

## License

O2A-authored code and documentation are available under `MIT OR Apache-2.0`.
Third-party dependencies retain their own licenses.
