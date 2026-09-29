# RGB 0.11.1 compatibility memo

Date: 2026-09-28. Worktree `wt-rgb011-spike`, branch `spike/rgb-0.11.1`, base `71471ee`. Evidence: `evidence/regtest-rgb011-compat-2026-09-28/`.

The regtest transitions succeeded. The dependency gate failed. O2A does not adopt rgb-protocol 0.11.1 on this result.

NO-GO

## Results

| Case | Expected | Observed | Result |
| --- | --- | --- | --- |
| C1 | O2A identity schema. Declarative `identity` right. 32-byte digest. Transitions `rotate`, `recover`, `revoke`. `validator = None`. Genesis carries a fresh O2A digest and validates. | Schema `rgb:sch:oqE1HKzG_NrzhV2M0tn~mkfJfic5ztF6iUr8s8YbdDE#ivan-robin-exotic`, different from the NIA schema. `validators_none true`. Contract `rgb:VWn__XsZ-pVGALkp-~x09Pp7-tsPls0w-AbOe8WU-V8odbX0`. Digest `cffe41dd6305f0461b5363faeae33ea0cb8c7dfef5d0680a74e24f7b15e94c57`. Report: `Consignment is valid`. | PASS |
| C2 | Assign that right to an external outpoint locked by the O2A seal script, then validate. The O2A script must match. The RGB seal type has no script field. | Outpoint `bc4f9ef9…b79fd1:0`. `o2a_script_match true`. `rgb_seal_has_no_script_field true`. Report: `Consignment is valid`. | PASS |
| C3 | Build a PSBT that spends the seal through the controller leaf. Fees come from a separate key-path input. Host output is Opret. Commit with the rgb-api PSBT layer. Report any wallet-ownership assumption with file and line. | Witness `78271447…b3baa6`. Transition `09cec6a8…061f5d7`. The spike put the seal input on the PSBT itself. See the wallet lines below. | PASS |
| C4 | Script-path sign, finalize by hand, broadcast, mine, then validate with two independent electrs resolvers. The two outputs are byte-identical. | Mined at height 109. `validators_agree true`. Report: `Consignment is valid`. Witness position `Mined` at height 109. | PASS |
| C5 | Recovery leaf. Bitcoin rejects the spend before BIP68 maturity and accepts it after. Record the heights. | Tip 109: `non-BIP68-final`. Same hex allowed at tip 117, broadcast at 117, mined at 118. Seal R was confirmed at 108, so inclusion is confirmation plus 10. | PASS |
| C6 | Plain spend of a seal with no commitment. Record the 0.11.1 validation text. | Mined at 119. Bitcoin spent the seal. The right remains `opout 3b62b0dc…124b2d/4101/0` on the spent outpoint. Witness `None`. Report: `Consignment is valid`. No error. | PASS |
| C7 | Two contracts on the same outpoint, same O2A genesis bytes. Both validate while the seal is unspent. After a spend that commits only contract 2, record contract 1. | `unspent_both true`, `same_digest true`, distinct ids `rgb:XF8a1bx5-…04G0uPw` and `rgb:CDoAVPTZ-…9rjcJjs`. After the spend, mined at 120, contract 1 still lists that outpoint, state `Void`, witness `None`, report `Consignment is valid`. | PASS |
| C8 | `cargo audit` and `cargo deny` pass. No release-candidate pin and no git pin. Record the crate count. | Audit exit 0, 139 crates, 1273 advisories. Lock has 139 packages, no git source, no rc, no alpha. `cargo deny` exit 4: advisories ok, bans ok, sources ok, licenses FAILED. Rejected crate: `hex_lit 0.1.1`, license `MITNFA`. | FAIL |
| C9 | Optional Tapret report. Do not broadcast. | Script-tree host: `use of taproot script descriptors is not yet supported.` Key-only host: `ok`, with a `TapretProof` (`partner_node` none, nonce 0). Not broadcast. | REPORT |

C5 and C6 behave as they did on the 2026-09-25 RGB 0.12 smoke. That smoke rejected the recovery spend with `non-BIP68-final`, allowed it later, and mined it at confirmation plus the CSV delay. Its plain-spend record printed no error and left the previous cell `Mined`. This run prints no error and leaves the consignment valid.

## Wallet lines for C3

These are in `rgb-api` 0.11.1 as published:

- `src/pay.rs:112-122`. `should_include` requires `wallet.filter_unspent().should_include` and a non-empty `stock.contract_assignments_for`.
- `src/pay.rs:535-537`. An empty selection returns `CompositionError::InsufficientState`.
- `src/pay.rs:554-556`. `set_rgb_close_method`, `set_as_unmodifiable`, and `rgb_embed` run on the PSBT.
- `src/pay.rs:570`. `transfer` calls `rgb_commit`.
- `src/wallet.rs:49-94`. `RgbWallet` wraps a `WalletProvider`. The loader that opens a `bp-wallet` wallet is behind `feature = "bp"` and starts at line 69.

The PSBT methods do not check that the spent outpoint belongs to a wallet. This spike never constructed `RgbWallet`. `rgb-api` default features are empty, so `bp` and `bdk` stayed off. Enabling `bp` would pull `bp-wallet`. Enabling `fs` would pull `bp-wallet/fs`.

## Why C8 fails

`bitcoin 0.32.102` depends on `hex_lit 0.1.1` as a normal dependency. `rgb-consensus 0.11.1` depends on that bitcoin crate. `hex_lit` declares `license = "MITNFA"`.

The maintainer decision of 2026-09-24 allows `Zlib` and keeps `MITNFA` disallowed. `DEMO-GATE.md` rule 6 says the same. The spike allow list includes `Zlib` and does not include `MITNFA`.

An earlier deny run also rejected `foldhash 0.1.5` because the spike list had omitted `Zlib`. That omission was corrected to match the published allow list, and deny was run again. The remaining failure is `MITNFA` alone. Duplicate crate versions, including `secp256k1` 0.29.1 and 0.33.1, are warnings. Bans still pass.

The parent 0.12 lock has no `hex_lit` entry. Its 2026-09-25 deny run exited 0. This MITNFA edge appears on the 0.11.1 graph.

## Effort to port the demo RGB adapter and the seal flow

The spike already contains the pieces a port would keep: the identity schema, genesis issue, external seal assignment, Opret PSBT commit, script-path finalizer, and an electrs resolver that reads block height from `getblockheader`.

Lifting that into a maintained module, still in its own Cargo workspace, is about three to five days of focused work. The seal scripts stay in `o2a-demo-core`. The module keeps calling the PSBT layer directly. Funding stays one transaction per output when several seals share an address. Electrs is started only after the index can answer `headers.subscribe`.

The license stop remains after that port. Adoption waits on a bitcoin crate line whose graph is free of MITNFA, or on a new maintainer decision. A new license decision is outside this check.

## Risks

- One Cargo graph that contains both 0.11.1 and 0.12 pulls two incompatible RGB lines. The parent workspace excludes `spikes/rgb-0.11.1` so that does not happen here.
- `rgb-lib` claims every wallet UTXO. This spike does not use it. A later port that calls it would fight O2A seal custody.
- `rgb-api` features `bp`, `fs`, and `bdk` pull `bp-wallet`. The working path leaves those features off.
- Electrs panics if a client calls `blockchain.headers.subscribe` before the index has a tip. Start it after the maturity mine, and connect after `indexed 102 blocks`.
- Core rejects a transaction that repeats an address. Seals that share the O2A script need one funding transaction each.
- Tapret commitment through the O2A script tree returned `use of taproot script descriptors is not yet supported.` The method that passed is `OpretFirst`.
- After a spend that commits only one of two contracts on the same outpoint, RGB 0.11.1 still validates the other contract and still lists its right on the spent outpoint. O2A has to enforce the ADR-0009 re-issue rule itself.
- The lock compiles `secp256k1` 0.29.1 for `bitcoin` and `o2a-demo-core`, and 0.33.1 for `rgb-consensus`. The spike crosses that boundary with byte arrays. `cargo deny` warns and does not fail the ban check.
- `cargo deny` 0.20 rejects this graph until `MITNFA` is gone or the maintainer changes the 2026-09-24 rule.

## Scope of this commit

The result is committed on `spike/rgb-0.11.1` only. It is not pushed. `feat/block0-rehearsal`, `main`, and `o2a-protocol` are unchanged. No mainnet transaction was created.
