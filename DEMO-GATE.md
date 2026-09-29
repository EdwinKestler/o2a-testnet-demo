# O2A Demo Gate

**Status:** mandatory boundary for the disposable testnet demonstration.
Specification authority: sibling `../o2a-protocol` at commit `0a8d54f30b431661adefdbf1d4cdb10a42eca47a`.

Maintained RGB line: rgb-protocol `0.11.1`, close method Opret. The adapter
lives in the separate workspace `spikes/rgb-0.11.1`. `o2a-demo-core` has no
RGB dependency and is the only default Cargo member. The RGB 0.12 RC3 adapter
remains in this workspace as archived legacy and is not part of the default
build. RGB 0.11 and RGB 0.12 are never resolved in one dependency graph.

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
6. Dependency licenses follow the 2026-09-28 assessment in
   `../o2a-protocol/docs/22-license-and-adoption-assessment.md`
   (decision D14 in `../o2a-protocol/docs/24-decision-audit-2026-09.md`).
   The blocking gate is `cargo deny check advisories bans sources`.
   `cargo deny check licenses` is report-only. A license outside the routine
   allowlist is entered in that document's license register.
   `MPL-2.0-no-copyleft-exception` remains a package-scoped exception for
   unmodified `base85 2.0.0`. MITNFA is assessed for `hex_lit 0.1.1` and is
   not added to the allowlist.
7. Evidence is append-only. A run is never overwritten or deleted; a
   correction or rerun gets a new dated bundle and manifest.
8. The demo site is the lowest authority level and never restates protocol rules; the O2A specification repository governs.
   Publication channels: OpenAI Sites, configured in .openai/hosting.json, is a private preview for internal review.

## Demo relaxations

- Implementation code is allowed in this repository.
- The maintained carrier is crates.io `rgb-consensus`, `rgb-schemas`,
  `rgb-ops`, `rgb-api`, and `rgb-psbt-utils` at exactly `0.11.1`, with
  `rgb-strict-encoding` and `rgb-strict-types` at `1.0.4`. The close method
  is Opret. `rgb-lib` is not a dependency. The `rgb-api` features `bp`,
  `bdk`, and `fs` stay off.
- The archived `rgb-runtime` pin remains RGB-WG `rgb` `v0.12.0-rc.3`, commit
  `a1e6b41524131f6d6f183b2235fdaacb5c1abb31`, with
  `default-features = false` and features `resolver-electrum` and `fs`,
  patched to EdwinKestler `f1e5a68992700ce208da81682ff092c0d5576e82`.
  That graph is not the default build. Its evidence and the patched fork
  stay in place. `rgb-wallet` is not a dependency.
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

The archived 0.12 seal-policy lineage is a Codex contract. Codex name
`O2ASealPolicyDemo`. Codex id
`21NiO7HR-YJ7lZHf-QqU5lFX-~hZoADX-_~cM26a-NeJbFdg#history-paper-polka`.
Issuer id
`21NiO7HR-YJ7lZHf-QqU5lFX-~hZoADX-_~cM26a-NeJbFdg/0#kq5rkg`.
Methods are `issue`, `rotateController`, `revoke`, and `recover`. Each
verifier is the same success program. RGB still validates no O2A semantics.

The maintained 0.11.1 schema is a different contract. Type library
`O2AIdentity011`, schema name `O2aIdentity`, global state `3101`, assignment
`4101`, transitions `8101` rotate, `8102` recover, and `8103` revoke.
Validators are unset. The compatibility run recorded schema id
`rgb:sch:oqE1HKzG_NrzhV2M0tn~mkfJfic5ztF6iUr8s8YbdDE#ivan-robin-exotic`.
That id is not the 0.12 Codex id.

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

- `O2A_NETWORK` selects one profile: `regtest` (the default), `signet`, or
  `mainnet`. Code outside that profile does not keep a second network table.
- Regtest is the development and evidence network. The profile uses network
  byte `4`, coin type `1'`, address prefix `bcrt`, and confirmation depth `1`.
- Signet is the rehearsal network. The profile uses network byte `3`, coin
  type `1'`, address prefix `tb`, and confirmation depth `1`. Signet
  identities are disposable.
- Mainnet is the block-0 profile: network byte `0`, coin type `0'`, address
  prefix `bc`, and confirmation depth `6`. `plan` and `verify` follow the
  profile and do not ask for the session lock. `genesis` and the one
  `official_name` claim use keys, so each of those commands also needs
  `--authorize-mainnet` and the typed word `mainnet`. Transitions are
  refused. Configured mainnet endpoints are read-only. This repository
  refuses every mainnet broadcast, including from an authorized session.
  The operator's wallet funds the seal. The block-0 identity is
  permanent. No mainnet identity network is running.
- Testnet and testnet4 stay recognized for verification. They are not
  selectable profiles.

## Non-negotiable identity warning

Regtest and signet identities created by this demo are disposable. Before
creating one, the CLI must display that warning and require explicit
acknowledgement. Demo keys, rehearsal identities, RGB state, proof packages,
and evidence must never be promoted to mainnet.

The mainnet block-0 identity is permanent. This repository does not describe
it as disposable and does not offer a reset that would mint it again under
the same identifier. Creating it needs the mainnet profile. `genesis` and
the `official_name` claim also need `--authorize-mainnet` and the typed
word `mainnet` in that session. The operator's wallet funds the seal.
