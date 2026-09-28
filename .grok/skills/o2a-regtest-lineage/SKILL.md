---
name: o2a-regtest-lineage
description: Run an O2A demo lineage on regtest with isolated infrastructure, independent verification and append-only evidence.
when-to-use: When producing lineage evidence or fixtures (F-series) on regtest.
---

# Regtest lineage

## Infrastructure
- Use a dedicated Compose project name (-p). Subnets must not collide: the base stack uses 172.30.30.0/24; overrides use `!override` on networks.<net>.ipam.config. Pass extra rpcallowip on the bitcoind command line in the override; never edit dev/bitcoin.conf or dev/compose.yaml.
- Never stop, remove or modify other projects (o2a-testnet-demo, o2a-phase0, o2a-seal-smoke, signet-infra) or any volume. List volumes and networks you created in the report.
- Record the starting height H0. Never invalidate a block at or below H0.

## Identities
- A DISTINCT entity index per identity (lesson: two "identities" once shared one root and one EntityID).
- Cross-check every seal address with Bitcoin Core getdescriptorinfo + deriveaddresses on the equivalent tr(NUMS,{...}) descriptor.

## Verification
- Every verification runs in two fresh validator directories, with the consignment imported. Outputs must be byte-identical.
- Reorg tests: invalidateblock / reconsiderblock above H0 only.

## Evidence
- New bundle evidence/<network>-<topic>-<YYYY-MM-DD>/ with RUN.md (header: disposable demo-lineage evidence; no Phase 0 gate closure), raw outputs, consignments, O2A objects and MANIFEST.sha256. Verify with `sha256sum -c`.
- Append-only: never edit an existing bundle. Corrections go in a new *-correction-<date> bundle.
- Scan for secrets before committing: `grep -rlaiE "xprv|tprv|mnemonic"` must be empty.
