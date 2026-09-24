# O2A Demo Gate

**Status:** mandatory boundary for the disposable testnet demonstration.
Specification authority: sibling `../o2a-protocol` at commit `3ca98ea9b60256f271e71fa89caa09448e804e87`.

## Rules carried from the specification

1. Verification always reports three distinct layers. Bitcoin establishes
   best-chain order, confirmation depth, and whether the referenced seal was
   spent. RGB validates the supplied contract history and consignment against
   its Bitcoin anchors. O2A validates canonical objects, controller
   authorization, key role, capability, and BIP340 signatures. A success in
   one layer never substitutes for another.
2. `o2a-demo-core` is the only signing and serialization implementation. Its
   rule is taken by reference from
   `../o2a-protocol/specs/canonical-encoding.md` and
   `../o2a-protocol/specs/cryptographic-profile.md` at the authority commit
   above. The CLI and RGB adapter call that implementation; they do not carry
   a second codec, hash, or signature rule.
3. Core evaluation is deterministic and never fetches the network, database,
   clock, DNS, HTTP, RPC, or Electrum. Adapters provide explicit evidence,
   validated RGB history, policy, and evaluation context as inputs.
4. Bitcoin payment keys have no O2A key role or key ID and never sign O2A
   objects.
5. RGB 0.11 is never mixed with RGB 0.12.
6. The dependency and distribution license policy in
   `../o2a-protocol/docs/22-license-and-adoption-assessment.md` applies.
   `MPL-2.0-no-copyleft-exception` is permitted only for unmodified
   `base85 2.0.0`; patching or forking it is a stop. MITNFA and every
   GPL/LGPL/AGPL expression are stops.
7. Evidence is append-only. A run is never overwritten or deleted; a
   correction or rerun gets a new dated bundle and manifest.
8. The demo site is the lowest authority level and never restates protocol rules; the O2A specification repository governs.

## Demo relaxations

- Implementation code is allowed in this repository.
- `rgb-runtime` is pinned to RGB-WG `rgb` `v0.12.0-rc.3`, commit
  `a1e6b41524131f6d6f183b2235fdaacb5c1abb31`, with
  `default-features = false` and features `resolver-electrum` and `fs`.
  `rgb-wallet` is not a dependency.
- Upstream patches are allowed only through `[patch.crates-io]` pointing to a
  named branch. Every patch must be recorded in `PATCHES.md` and cite the
  matching item in `../o2a-protocol/docs/upstream-needs.md`.
- Custody-acceptance and recovery fixtures may use real RGB prior state
  produced on regtest. Synthetic authorizing state is never accepted.
- The derivation profile is used as **demo-stable v0.1**. It is not frozen and
  creates no production-compatible identity commitment.

## Networks

- Regtest is used now for development and append-only evidence.
- The default public Bitcoin signet is the later live target. It uses network
  byte `3`, coin type `1'`, and confirmation depth `1`.
- Testnet3 is not wired. Mainnet is out of scope.

## Non-negotiable identity warning

Every identity created by this demo is disposable. Before creation, the CLI
must display that warning and require explicit acknowledgement. Demo keys,
identities, RGB state, proof packages, and evidence must never be promoted to
mainnet or treated as persistent production identities.
