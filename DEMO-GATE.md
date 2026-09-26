# O2A Demo Gate

**Status:** mandatory boundary for the disposable testnet demonstration.
Specification authority: sibling `../o2a-protocol` at commit `c7b08716d017d1f6125e6a728fb098673a09d433`.

## Rules carried from the specification

1. Verification always reports three distinct layers. Bitcoin establishes
   best-chain order, confirmation depth, and whether the referenced seal was
   spent. RGB validates the supplied contract history and consignment against
   its Bitcoin anchors. O2A validates canonical objects, controller
   authorization, key role, capability, and BIP340 signatures. A success in
   one layer never substitutes for another.
2. `o2a-demo-core` is the only signing and serialization implementation. Its
   rule is taken by reference from
   `../o2a-protocol/specs/canonical-encoding.md`,
   `../o2a-protocol/specs/cryptographic-profile.md`,
   `../o2a-protocol/specs/key-derivation-profile.md`, and
   `../o2a-protocol/specs/rgb-identity-contract.md` at the authority commit
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
   Publication channels: OpenAI Sites, configured in .openai/hosting.json, is a private preview for internal review.

## Demo relaxations

- Implementation code is allowed in this repository.
- `rgb-runtime` is pinned to RGB-WG `rgb` `v0.12.0-rc.3`, commit
  `a1e6b41524131f6d6f183b2235fdaacb5c1abb31`, with
  `default-features = false` and features `resolver-electrum` and `fs`.
  `rgb-wallet` is not a dependency.
- Upstream patches are allowed only through `[patch.crates-io]` pointing to a
  named branch. Every patch must be recorded in `PATCHES.md` and cite the
  matching item in `../o2a-protocol/docs/upstream-needs.md`. The two `bp-std`
  script-path issues stay unpatched. A seal spend uses the smoke-proven
  workarounds: manual script-path finalization, and an explicit empty
  `final_script_sig`. See `../o2a-protocol/docs/upstream-needs.md`, BP-WG
  `bp-std` items 1 and 2.
- Custody-acceptance and recovery fixtures may use real RGB prior state
  produced on regtest. Synthetic authorizing state is never accepted.
- The derivation profile is used as **demo-stable v0.1**. It is not frozen and
  creates no production-compatible identity commitment.

## Seal-policy contract type

The seal-policy lineage is a new RGB contract type. Codex name
`O2ASealPolicyDemo`. Codex id
`21NiO7HR-YJ7lZHf-QqU5lFX-~hZoADX-_~cM26a-NeJbFdg#history-paper-polka`.
Issuer id
`21NiO7HR-YJ7lZHf-QqU5lFX-~hZoADX-_~cM26a-NeJbFdg/0#kq5rkg`.
Methods are `issue`, `rotateController`, `revoke`, and `recover`. Each
verifier is the same success program. RGB still validates no O2A semantics.

## Recovery thresholds

Vector objects keep the fixed spec inputs: recovery threshold 1 and
`delay_blocks` 6. The demo lineage uses a 2-of-3 recovery set and
`delay_blocks` 10. Phase B cross-checks every lineage seal address with
Bitcoin Core `getdescriptorinfo` and `deriveaddresses`, and records that
output.

## Open demo operations

These identity-transition operations are open. `o2a-demo-core` rejects each
signed payload with `unsupported in demo`.

- Operation 2, recovery-policy change.
- Operation 4, custody transfer.

Operation 3 remains the recovery-authorization operation. Inside an identity
transition it is invalid.

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
