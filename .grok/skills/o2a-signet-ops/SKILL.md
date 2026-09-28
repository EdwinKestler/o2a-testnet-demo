---
name: o2a-signet-ops
description: Operate the O2A signet infrastructure and rehearsal wallet safely.
when-to-use: Only when explicitly asked to run signet work.
disable-model-invocation: true
---

# Signet operations

- Infrastructure: the signet-infra Compose project (bitcoind + electrs, wallet override enabled). Confirm the node tip matches a public signet explorer before starting.
- The toolchain reaches signet electrs through dev/compose.signet-route.yaml. Do not change dev/compose.yaml.
- Rehearsal wallet: generated from a FRESH rehearsal seed, never the published unsafe seed. The seed never enters the repository or evidence.
- Funding: public signet faucets usually need a manual browser challenge. Ask the maintainer instead of trying to bypass it. Budget about 20,000 sats per full rehearsal.
- Rehearse mainnet timing: wait for 6 confirmations even though signet's demo depth is 1.
- NO mainnet keys, addresses, transactions or funds, under any wording, unless ADR-0009 is Accepted AND the maintainer explicitly authorizes mainnet in the current session.
