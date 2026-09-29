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

## Verdict revised: GO (2026-09-28)

Maintainer license-policy decision D14, recorded on 2026-09-28 in `o2a-protocol` `docs/24-decision-audit-2026-09.md`, treats dependency licenses as an assessment. Advisories, bans, and sources stay blocking. `cargo deny check licenses` is report-only. `hex_lit 0.1.1` (MITNFA), pulled through `bitcoin 0.32.102` by `rgb-consensus 0.11.1`, is entered as `accepted` in `o2a-protocol` `docs/22-license-and-adoption-assessment.md`. It is not added to the allowlist.

The blocking rerun exited 0:

```text
$ cargo deny check advisories bans sources
warning[duplicate]: found 2 duplicate entries for crate 'block-buffer'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:21:1
   │  
21 │ ╭ block-buffer 0.10.4 registry+https://github.com/rust-lang/crates.io-index
22 │ │ block-buffer 0.12.1 registry+https://github.com/rust-lang/crates.io-index
   │ ╰─────────────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ block-buffer v0.10.4
     └── digest v0.10.7
         └── sha2 v0.10.9
             └── baid64 v0.4.1
                 ├── rgb-aluvm v0.11.1
                 │   ├── rgb-consensus v0.11.1
                 │   │   ├── rgb-invoicing v0.11.1
                 │   │   │   └── rgb-ops v0.11.1
                 │   │   │       ├── rgb-api v0.11.1
                 │   │   │       │   └── rgb011-check v0.1.0
                 │   │   │       ├── rgb-psbt-utils v0.11.1
                 │   │   │       │   ├── rgb-api v0.11.1 (*)
                 │   │   │       │   └── rgb011-check v0.1.0 (*)
                 │   │   │       ├── rgb-schemas v0.11.1
                 │   │   │       │   └── rgb011-check v0.1.0 (*)
                 │   │   │       └── rgb011-check v0.1.0 (*)
                 │   │   ├── rgb-ops v0.11.1 (*)
                 │   │   └── rgb011-check v0.1.0 (*)
                 │   ├── rgb-ops v0.11.1 (*)
                 │   └── rgb-schemas v0.11.1 (*)
                 ├── rgb-api v0.11.1 (*)
                 ├── rgb-ascii-armor v1.0.4
                 │   ├── rgb-aluvm v0.11.1 (*)
                 │   ├── rgb-ops v0.11.1 (*)
                 │   └── rgb-strict-types v1.0.4
                 │       ├── rgb-aluvm v0.11.1 (*)
                 │       ├── rgb-api v0.11.1 (*)
                 │       ├── rgb-consensus v0.11.1 (*)
                 │       ├── rgb-invoicing v0.11.1 (*)
                 │       ├── rgb-ops v0.11.1 (*)
                 │       ├── rgb-schemas v0.11.1 (*)
                 │       └── rgb011-check v0.1.0 (*)
                 ├── rgb-consensus v0.11.1 (*)
                 ├── rgb-invoicing v0.11.1 (*)
                 ├── rgb-ops v0.11.1 (*)
                 └── rgb-strict-types v1.0.4 (*)
   ├ block-buffer v0.12.1
     └── digest v0.11.3
         ├── ripemd v0.2.0
         │   ├── rgb-aluvm v0.11.1
         │   │   ├── rgb-consensus v0.11.1
         │   │   │   ├── rgb-invoicing v0.11.1
         │   │   │   │   └── rgb-ops v0.11.1
         │   │   │   │       ├── rgb-api v0.11.1
         │   │   │   │       │   └── rgb011-check v0.1.0
         │   │   │   │       ├── rgb-psbt-utils v0.11.1
         │   │   │   │       │   ├── rgb-api v0.11.1 (*)
         │   │   │   │       │   └── rgb011-check v0.1.0 (*)
         │   │   │   │       ├── rgb-schemas v0.11.1
         │   │   │   │       │   └── rgb011-check v0.1.0 (*)
         │   │   │   │       └── rgb011-check v0.1.0 (*)
         │   │   │   ├── rgb-ops v0.11.1 (*)
         │   │   │   └── rgb011-check v0.1.0 (*)
         │   │   ├── rgb-ops v0.11.1 (*)
         │   │   └── rgb-schemas v0.11.1 (*)
         │   └── rgb-consensus v0.11.1 (*)
         └── sha2 v0.11.0
             ├── rgb-aluvm v0.11.1 (*)
             ├── rgb-ascii-armor v1.0.4
             │   ├── rgb-aluvm v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-strict-types v1.0.4
             │       ├── rgb-aluvm v0.11.1 (*)
             │       ├── rgb-api v0.11.1 (*)
             │       ├── rgb-consensus v0.11.1 (*)
             │       ├── rgb-invoicing v0.11.1 (*)
             │       ├── rgb-ops v0.11.1 (*)
             │       ├── rgb-schemas v0.11.1 (*)
             │       └── rgb011-check v0.1.0 (*)
             ├── rgb-consensus v0.11.1 (*)
             └── rgb-strict-types v1.0.4 (*)

warning[duplicate]: found 2 duplicate entries for crate 'cpufeatures'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:31:1
   │  
31 │ ╭ cpufeatures 0.2.17 registry+https://github.com/rust-lang/crates.io-index
32 │ │ cpufeatures 0.3.1 registry+https://github.com/rust-lang/crates.io-index
   │ ╰───────────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ cpufeatures v0.2.17
     └── sha2 v0.10.9
         └── baid64 v0.4.1
             ├── rgb-aluvm v0.11.1
             │   ├── rgb-consensus v0.11.1
             │   │   ├── rgb-invoicing v0.11.1
             │   │   │   └── rgb-ops v0.11.1
             │   │   │       ├── rgb-api v0.11.1
             │   │   │       │   └── rgb011-check v0.1.0
             │   │   │       ├── rgb-psbt-utils v0.11.1
             │   │   │       │   ├── rgb-api v0.11.1 (*)
             │   │   │       │   └── rgb011-check v0.1.0 (*)
             │   │   │       ├── rgb-schemas v0.11.1
             │   │   │       │   └── rgb011-check v0.1.0 (*)
             │   │   │       └── rgb011-check v0.1.0 (*)
             │   │   ├── rgb-ops v0.11.1 (*)
             │   │   └── rgb011-check v0.1.0 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-schemas v0.11.1 (*)
             ├── rgb-api v0.11.1 (*)
             ├── rgb-ascii-armor v1.0.4
             │   ├── rgb-aluvm v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-strict-types v1.0.4
             │       ├── rgb-aluvm v0.11.1 (*)
             │       ├── rgb-api v0.11.1 (*)
             │       ├── rgb-consensus v0.11.1 (*)
             │       ├── rgb-invoicing v0.11.1 (*)
             │       ├── rgb-ops v0.11.1 (*)
             │       ├── rgb-schemas v0.11.1 (*)
             │       └── rgb011-check v0.1.0 (*)
             ├── rgb-consensus v0.11.1 (*)
             ├── rgb-invoicing v0.11.1 (*)
             ├── rgb-ops v0.11.1 (*)
             └── rgb-strict-types v1.0.4 (*)
   ├ cpufeatures v0.3.1
     ├── blake3 v1.8.7
     │   └── rgb-aluvm v0.11.1
     │       ├── rgb-consensus v0.11.1
     │       │   ├── rgb-invoicing v0.11.1
     │       │   │   └── rgb-ops v0.11.1
     │       │   │       ├── rgb-api v0.11.1
     │       │   │       │   └── rgb011-check v0.1.0
     │       │   │       ├── rgb-psbt-utils v0.11.1
     │       │   │       │   ├── rgb-api v0.11.1 (*)
     │       │   │       │   └── rgb011-check v0.1.0 (*)
     │       │   │       ├── rgb-schemas v0.11.1
     │       │   │       │   └── rgb011-check v0.1.0 (*)
     │       │   │       └── rgb011-check v0.1.0 (*)
     │       │   ├── rgb-ops v0.11.1 (*)
     │       │   └── rgb011-check v0.1.0 (*)
     │       ├── rgb-ops v0.11.1 (*)
     │       └── rgb-schemas v0.11.1 (*)
     └── sha2 v0.11.0
         ├── rgb-aluvm v0.11.1 (*)
         ├── rgb-ascii-armor v1.0.4
         │   ├── rgb-aluvm v0.11.1 (*)
         │   ├── rgb-ops v0.11.1 (*)
         │   └── rgb-strict-types v1.0.4
         │       ├── rgb-aluvm v0.11.1 (*)
         │       ├── rgb-api v0.11.1 (*)
         │       ├── rgb-consensus v0.11.1 (*)
         │       ├── rgb-invoicing v0.11.1 (*)
         │       ├── rgb-ops v0.11.1 (*)
         │       ├── rgb-schemas v0.11.1 (*)
         │       └── rgb011-check v0.1.0 (*)
         ├── rgb-consensus v0.11.1 (*)
         └── rgb-strict-types v1.0.4 (*)

warning[duplicate]: found 2 duplicate entries for crate 'crypto-common'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:34:1
   │  
34 │ ╭ crypto-common 0.1.7 registry+https://github.com/rust-lang/crates.io-index
35 │ │ crypto-common 0.2.2 registry+https://github.com/rust-lang/crates.io-index
   │ ╰─────────────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ crypto-common v0.1.7
     └── digest v0.10.7
         └── sha2 v0.10.9
             └── baid64 v0.4.1
                 ├── rgb-aluvm v0.11.1
                 │   ├── rgb-consensus v0.11.1
                 │   │   ├── rgb-invoicing v0.11.1
                 │   │   │   └── rgb-ops v0.11.1
                 │   │   │       ├── rgb-api v0.11.1
                 │   │   │       │   └── rgb011-check v0.1.0
                 │   │   │       ├── rgb-psbt-utils v0.11.1
                 │   │   │       │   ├── rgb-api v0.11.1 (*)
                 │   │   │       │   └── rgb011-check v0.1.0 (*)
                 │   │   │       ├── rgb-schemas v0.11.1
                 │   │   │       │   └── rgb011-check v0.1.0 (*)
                 │   │   │       └── rgb011-check v0.1.0 (*)
                 │   │   ├── rgb-ops v0.11.1 (*)
                 │   │   └── rgb011-check v0.1.0 (*)
                 │   ├── rgb-ops v0.11.1 (*)
                 │   └── rgb-schemas v0.11.1 (*)
                 ├── rgb-api v0.11.1 (*)
                 ├── rgb-ascii-armor v1.0.4
                 │   ├── rgb-aluvm v0.11.1 (*)
                 │   ├── rgb-ops v0.11.1 (*)
                 │   └── rgb-strict-types v1.0.4
                 │       ├── rgb-aluvm v0.11.1 (*)
                 │       ├── rgb-api v0.11.1 (*)
                 │       ├── rgb-consensus v0.11.1 (*)
                 │       ├── rgb-invoicing v0.11.1 (*)
                 │       ├── rgb-ops v0.11.1 (*)
                 │       ├── rgb-schemas v0.11.1 (*)
                 │       └── rgb011-check v0.1.0 (*)
                 ├── rgb-consensus v0.11.1 (*)
                 ├── rgb-invoicing v0.11.1 (*)
                 ├── rgb-ops v0.11.1 (*)
                 └── rgb-strict-types v1.0.4 (*)
   ├ crypto-common v0.2.2
     └── digest v0.11.3
         ├── ripemd v0.2.0
         │   ├── rgb-aluvm v0.11.1
         │   │   ├── rgb-consensus v0.11.1
         │   │   │   ├── rgb-invoicing v0.11.1
         │   │   │   │   └── rgb-ops v0.11.1
         │   │   │   │       ├── rgb-api v0.11.1
         │   │   │   │       │   └── rgb011-check v0.1.0
         │   │   │   │       ├── rgb-psbt-utils v0.11.1
         │   │   │   │       │   ├── rgb-api v0.11.1 (*)
         │   │   │   │       │   └── rgb011-check v0.1.0 (*)
         │   │   │   │       ├── rgb-schemas v0.11.1
         │   │   │   │       │   └── rgb011-check v0.1.0 (*)
         │   │   │   │       └── rgb011-check v0.1.0 (*)
         │   │   │   ├── rgb-ops v0.11.1 (*)
         │   │   │   └── rgb011-check v0.1.0 (*)
         │   │   ├── rgb-ops v0.11.1 (*)
         │   │   └── rgb-schemas v0.11.1 (*)
         │   └── rgb-consensus v0.11.1 (*)
         └── sha2 v0.11.0
             ├── rgb-aluvm v0.11.1 (*)
             ├── rgb-ascii-armor v1.0.4
             │   ├── rgb-aluvm v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-strict-types v1.0.4
             │       ├── rgb-aluvm v0.11.1 (*)
             │       ├── rgb-api v0.11.1 (*)
             │       ├── rgb-consensus v0.11.1 (*)
             │       ├── rgb-invoicing v0.11.1 (*)
             │       ├── rgb-ops v0.11.1 (*)
             │       ├── rgb-schemas v0.11.1 (*)
             │       └── rgb011-check v0.1.0 (*)
             ├── rgb-consensus v0.11.1 (*)
             └── rgb-strict-types v1.0.4 (*)

warning[duplicate]: found 2 duplicate entries for crate 'digest'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:37:1
   │  
37 │ ╭ digest 0.10.7 registry+https://github.com/rust-lang/crates.io-index
38 │ │ digest 0.11.3 registry+https://github.com/rust-lang/crates.io-index
   │ ╰───────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ digest v0.10.7
     └── sha2 v0.10.9
         └── baid64 v0.4.1
             ├── rgb-aluvm v0.11.1
             │   ├── rgb-consensus v0.11.1
             │   │   ├── rgb-invoicing v0.11.1
             │   │   │   └── rgb-ops v0.11.1
             │   │   │       ├── rgb-api v0.11.1
             │   │   │       │   └── rgb011-check v0.1.0
             │   │   │       ├── rgb-psbt-utils v0.11.1
             │   │   │       │   ├── rgb-api v0.11.1 (*)
             │   │   │       │   └── rgb011-check v0.1.0 (*)
             │   │   │       ├── rgb-schemas v0.11.1
             │   │   │       │   └── rgb011-check v0.1.0 (*)
             │   │   │       └── rgb011-check v0.1.0 (*)
             │   │   ├── rgb-ops v0.11.1 (*)
             │   │   └── rgb011-check v0.1.0 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-schemas v0.11.1 (*)
             ├── rgb-api v0.11.1 (*)
             ├── rgb-ascii-armor v1.0.4
             │   ├── rgb-aluvm v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-strict-types v1.0.4
             │       ├── rgb-aluvm v0.11.1 (*)
             │       ├── rgb-api v0.11.1 (*)
             │       ├── rgb-consensus v0.11.1 (*)
             │       ├── rgb-invoicing v0.11.1 (*)
             │       ├── rgb-ops v0.11.1 (*)
             │       ├── rgb-schemas v0.11.1 (*)
             │       └── rgb011-check v0.1.0 (*)
             ├── rgb-consensus v0.11.1 (*)
             ├── rgb-invoicing v0.11.1 (*)
             ├── rgb-ops v0.11.1 (*)
             └── rgb-strict-types v1.0.4 (*)
   ├ digest v0.11.3
     ├── ripemd v0.2.0
     │   ├── rgb-aluvm v0.11.1
     │   │   ├── rgb-consensus v0.11.1
     │   │   │   ├── rgb-invoicing v0.11.1
     │   │   │   │   └── rgb-ops v0.11.1
     │   │   │   │       ├── rgb-api v0.11.1
     │   │   │   │       │   └── rgb011-check v0.1.0
     │   │   │   │       ├── rgb-psbt-utils v0.11.1
     │   │   │   │       │   ├── rgb-api v0.11.1 (*)
     │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │   │   │       ├── rgb-schemas v0.11.1
     │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │   │   │       └── rgb011-check v0.1.0 (*)
     │   │   │   ├── rgb-ops v0.11.1 (*)
     │   │   │   └── rgb011-check v0.1.0 (*)
     │   │   ├── rgb-ops v0.11.1 (*)
     │   │   └── rgb-schemas v0.11.1 (*)
     │   └── rgb-consensus v0.11.1 (*)
     └── sha2 v0.11.0
         ├── rgb-aluvm v0.11.1 (*)
         ├── rgb-ascii-armor v1.0.4
         │   ├── rgb-aluvm v0.11.1 (*)
         │   ├── rgb-ops v0.11.1 (*)
         │   └── rgb-strict-types v1.0.4
         │       ├── rgb-aluvm v0.11.1 (*)
         │       ├── rgb-api v0.11.1 (*)
         │       ├── rgb-consensus v0.11.1 (*)
         │       ├── rgb-invoicing v0.11.1 (*)
         │       ├── rgb-ops v0.11.1 (*)
         │       ├── rgb-schemas v0.11.1 (*)
         │       └── rgb011-check v0.1.0 (*)
         ├── rgb-consensus v0.11.1 (*)
         └── rgb-strict-types v1.0.4 (*)

warning[duplicate]: found 2 duplicate entries for crate 'getrandom'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:49:1
   │  
49 │ ╭ getrandom 0.2.17 registry+https://github.com/rust-lang/crates.io-index
50 │ │ getrandom 0.3.4 registry+https://github.com/rust-lang/crates.io-index
   │ ╰─────────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ getrandom v0.2.17
     ├── rand_core v0.6.4
     │   ├── rand v0.8.8
     │   │   └── amplify v4.8.1
     │   │       ├── baid64 v0.4.1
     │   │       │   ├── rgb-aluvm v0.11.1
     │   │       │   │   ├── rgb-consensus v0.11.1
     │   │       │   │   │   ├── rgb-invoicing v0.11.1
     │   │       │   │   │   │   └── rgb-ops v0.11.1
     │   │       │   │   │   │       ├── rgb-api v0.11.1
     │   │       │   │   │   │       │   └── rgb011-check v0.1.0
     │   │       │   │   │   │       ├── rgb-psbt-utils v0.11.1
     │   │       │   │   │   │       │   ├── rgb-api v0.11.1 (*)
     │   │       │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │       │   │   │   │       ├── rgb-schemas v0.11.1
     │   │       │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │       │   │   │   │       └── rgb011-check v0.1.0 (*)
     │   │       │   │   │   ├── rgb-ops v0.11.1 (*)
     │   │       │   │   │   └── rgb011-check v0.1.0 (*)
     │   │       │   │   ├── rgb-ops v0.11.1 (*)
     │   │       │   │   └── rgb-schemas v0.11.1 (*)
     │   │       │   ├── rgb-api v0.11.1 (*)
     │   │       │   ├── rgb-ascii-armor v1.0.4
     │   │       │   │   ├── rgb-aluvm v0.11.1 (*)
     │   │       │   │   ├── rgb-ops v0.11.1 (*)
     │   │       │   │   └── rgb-strict-types v1.0.4
     │   │       │   │       ├── rgb-aluvm v0.11.1 (*)
     │   │       │   │       ├── rgb-api v0.11.1 (*)
     │   │       │   │       ├── rgb-consensus v0.11.1 (*)
     │   │       │   │       ├── rgb-invoicing v0.11.1 (*)
     │   │       │   │       ├── rgb-ops v0.11.1 (*)
     │   │       │   │       ├── rgb-schemas v0.11.1 (*)
     │   │       │   │       └── rgb011-check v0.1.0 (*)
     │   │       │   ├── rgb-consensus v0.11.1 (*)
     │   │       │   ├── rgb-invoicing v0.11.1 (*)
     │   │       │   ├── rgb-ops v0.11.1 (*)
     │   │       │   └── rgb-strict-types v1.0.4 (*)
     │   │       ├── nonasync v0.1.3
     │   │       │   ├── rgb-api v0.11.1 (*)
     │   │       │   └── rgb-ops v0.11.1 (*)
     │   │       ├── rgb-aluvm v0.11.1 (*)
     │   │       ├── rgb-api v0.11.1 (*)
     │   │       ├── rgb-ascii-armor v1.0.4 (*)
     │   │       ├── rgb-consensus v0.11.1 (*)
     │   │       ├── rgb-invoicing v0.11.1 (*)
     │   │       ├── rgb-ops v0.11.1 (*)
     │   │       ├── rgb-psbt-utils v0.11.1 (*)
     │   │       ├── rgb-schemas v0.11.1 (*)
     │   │       ├── rgb-strict-encoding v1.0.4
     │   │       │   ├── rgb-aluvm v0.11.1 (*)
     │   │       │   ├── rgb-ascii-armor v1.0.4 (*)
     │   │       │   ├── rgb-consensus v0.11.1 (*)
     │   │       │   ├── rgb-invoicing v0.11.1 (*)
     │   │       │   ├── rgb-ops v0.11.1 (*)
     │   │       │   ├── rgb-psbt-utils v0.11.1 (*)
     │   │       │   ├── rgb-strict-types v1.0.4 (*)
     │   │       │   └── rgb011-check v0.1.0 (*)
     │   │       ├── rgb-strict-types v1.0.4 (*)
     │   │       └── rgb011-check v0.1.0 (*)
     │   └── rand_chacha v0.3.1
     │       └── rand v0.8.8 (*)
     └── rgb-consensus v0.11.1 (*)
   ├ getrandom v0.3.4
     ├── rand_core v0.9.5
     │   ├── rand v0.9.5
     │   │   ├── rgb-consensus v0.11.1
     │   │   │   ├── rgb-invoicing v0.11.1
     │   │   │   │   └── rgb-ops v0.11.1
     │   │   │   │       ├── rgb-api v0.11.1
     │   │   │   │       │   └── rgb011-check v0.1.0
     │   │   │   │       ├── rgb-psbt-utils v0.11.1
     │   │   │   │       │   ├── rgb-api v0.11.1 (*)
     │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │   │   │       ├── rgb-schemas v0.11.1
     │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │   │   │       └── rgb011-check v0.1.0 (*)
     │   │   │   ├── rgb-ops v0.11.1 (*)
     │   │   │   └── rgb011-check v0.1.0 (*)
     │   │   ├── rgb-ops v0.11.1 (*)
     │   │   └── secp256k1 v0.33.1
     │   │       └── rgb-consensus v0.11.1 (*)
     │   └── rand_chacha v0.9.0
     │       └── rand v0.9.5 (*)
     ├── rgb-api v0.11.1 (*)
     ├── rgb-consensus v0.11.1 (*)
     ├── rgb-ops v0.11.1 (*)
     └── rgb-psbt-utils v0.11.1 (*)

warning[duplicate]: found 2 duplicate entries for crate 'hashbrown'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:52:1
   │  
52 │ ╭ hashbrown 0.15.5 registry+https://github.com/rust-lang/crates.io-index
53 │ │ hashbrown 0.17.1 registry+https://github.com/rust-lang/crates.io-index
   │ ╰──────────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ hashbrown v0.15.5
     └── petgraph v0.8.3
         └── daggy v0.9.0
             └── rgb-consensus v0.11.1
                 ├── rgb-invoicing v0.11.1
                 │   └── rgb-ops v0.11.1
                 │       ├── rgb-api v0.11.1
                 │       │   └── rgb011-check v0.1.0
                 │       ├── rgb-psbt-utils v0.11.1
                 │       │   ├── rgb-api v0.11.1 (*)
                 │       │   └── rgb011-check v0.1.0 (*)
                 │       ├── rgb-schemas v0.11.1
                 │       │   └── rgb011-check v0.1.0 (*)
                 │       └── rgb011-check v0.1.0 (*)
                 ├── rgb-ops v0.11.1 (*)
                 └── rgb011-check v0.1.0 (*)
   ├ hashbrown v0.17.1
     └── indexmap v2.14.2
         ├── petgraph v0.8.3
         │   └── daggy v0.9.0
         │       └── rgb-consensus v0.11.1
         │           ├── rgb-invoicing v0.11.1
         │           │   └── rgb-ops v0.11.1
         │           │       ├── rgb-api v0.11.1
         │           │       │   └── rgb011-check v0.1.0
         │           │       ├── rgb-psbt-utils v0.11.1
         │           │       │   ├── rgb-api v0.11.1 (*)
         │           │       │   └── rgb011-check v0.1.0 (*)
         │           │       ├── rgb-schemas v0.11.1
         │           │       │   └── rgb011-check v0.1.0 (*)
         │           │       └── rgb011-check v0.1.0 (*)
         │           ├── rgb-ops v0.11.1 (*)
         │           └── rgb011-check v0.1.0 (*)
         ├── rgb-api v0.11.1 (*)
         ├── rgb-invoicing v0.11.1 (*)
         └── rgb-strict-types v1.0.4
             ├── rgb-aluvm v0.11.1
             │   ├── rgb-consensus v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-schemas v0.11.1 (*)
             ├── rgb-api v0.11.1 (*)
             ├── rgb-consensus v0.11.1 (*)
             ├── rgb-invoicing v0.11.1 (*)
             ├── rgb-ops v0.11.1 (*)
             ├── rgb-schemas v0.11.1 (*)
             └── rgb011-check v0.1.0 (*)

warning[duplicate]: found 2 duplicate entries for crate 'rand'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:79:1
   │  
79 │ ╭ rand 0.8.8 registry+https://github.com/rust-lang/crates.io-index
80 │ │ rand 0.9.5 registry+https://github.com/rust-lang/crates.io-index
   │ ╰────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ rand v0.8.8
     └── amplify v4.8.1
         ├── baid64 v0.4.1
         │   ├── rgb-aluvm v0.11.1
         │   │   ├── rgb-consensus v0.11.1
         │   │   │   ├── rgb-invoicing v0.11.1
         │   │   │   │   └── rgb-ops v0.11.1
         │   │   │   │       ├── rgb-api v0.11.1
         │   │   │   │       │   └── rgb011-check v0.1.0
         │   │   │   │       ├── rgb-psbt-utils v0.11.1
         │   │   │   │       │   ├── rgb-api v0.11.1 (*)
         │   │   │   │       │   └── rgb011-check v0.1.0 (*)
         │   │   │   │       ├── rgb-schemas v0.11.1
         │   │   │   │       │   └── rgb011-check v0.1.0 (*)
         │   │   │   │       └── rgb011-check v0.1.0 (*)
         │   │   │   ├── rgb-ops v0.11.1 (*)
         │   │   │   └── rgb011-check v0.1.0 (*)
         │   │   ├── rgb-ops v0.11.1 (*)
         │   │   └── rgb-schemas v0.11.1 (*)
         │   ├── rgb-api v0.11.1 (*)
         │   ├── rgb-ascii-armor v1.0.4
         │   │   ├── rgb-aluvm v0.11.1 (*)
         │   │   ├── rgb-ops v0.11.1 (*)
         │   │   └── rgb-strict-types v1.0.4
         │   │       ├── rgb-aluvm v0.11.1 (*)
         │   │       ├── rgb-api v0.11.1 (*)
         │   │       ├── rgb-consensus v0.11.1 (*)
         │   │       ├── rgb-invoicing v0.11.1 (*)
         │   │       ├── rgb-ops v0.11.1 (*)
         │   │       ├── rgb-schemas v0.11.1 (*)
         │   │       └── rgb011-check v0.1.0 (*)
         │   ├── rgb-consensus v0.11.1 (*)
         │   ├── rgb-invoicing v0.11.1 (*)
         │   ├── rgb-ops v0.11.1 (*)
         │   └── rgb-strict-types v1.0.4 (*)
         ├── nonasync v0.1.3
         │   ├── rgb-api v0.11.1 (*)
         │   └── rgb-ops v0.11.1 (*)
         ├── rgb-aluvm v0.11.1 (*)
         ├── rgb-api v0.11.1 (*)
         ├── rgb-ascii-armor v1.0.4 (*)
         ├── rgb-consensus v0.11.1 (*)
         ├── rgb-invoicing v0.11.1 (*)
         ├── rgb-ops v0.11.1 (*)
         ├── rgb-psbt-utils v0.11.1 (*)
         ├── rgb-schemas v0.11.1 (*)
         ├── rgb-strict-encoding v1.0.4
         │   ├── rgb-aluvm v0.11.1 (*)
         │   ├── rgb-ascii-armor v1.0.4 (*)
         │   ├── rgb-consensus v0.11.1 (*)
         │   ├── rgb-invoicing v0.11.1 (*)
         │   ├── rgb-ops v0.11.1 (*)
         │   ├── rgb-psbt-utils v0.11.1 (*)
         │   ├── rgb-strict-types v1.0.4 (*)
         │   └── rgb011-check v0.1.0 (*)
         ├── rgb-strict-types v1.0.4 (*)
         └── rgb011-check v0.1.0 (*)
   ├ rand v0.9.5
     ├── rgb-consensus v0.11.1
     │   ├── rgb-invoicing v0.11.1
     │   │   └── rgb-ops v0.11.1
     │   │       ├── rgb-api v0.11.1
     │   │       │   └── rgb011-check v0.1.0
     │   │       ├── rgb-psbt-utils v0.11.1
     │   │       │   ├── rgb-api v0.11.1 (*)
     │   │       │   └── rgb011-check v0.1.0 (*)
     │   │       ├── rgb-schemas v0.11.1
     │   │       │   └── rgb011-check v0.1.0 (*)
     │   │       └── rgb011-check v0.1.0 (*)
     │   ├── rgb-ops v0.11.1 (*)
     │   └── rgb011-check v0.1.0 (*)
     ├── rgb-ops v0.11.1 (*)
     └── secp256k1 v0.33.1
         └── rgb-consensus v0.11.1 (*)

warning[duplicate]: found 2 duplicate entries for crate 'rand_chacha'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:81:1
   │  
81 │ ╭ rand_chacha 0.3.1 registry+https://github.com/rust-lang/crates.io-index
82 │ │ rand_chacha 0.9.0 registry+https://github.com/rust-lang/crates.io-index
   │ ╰───────────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ rand_chacha v0.3.1
     └── rand v0.8.8
         └── amplify v4.8.1
             ├── baid64 v0.4.1
             │   ├── rgb-aluvm v0.11.1
             │   │   ├── rgb-consensus v0.11.1
             │   │   │   ├── rgb-invoicing v0.11.1
             │   │   │   │   └── rgb-ops v0.11.1
             │   │   │   │       ├── rgb-api v0.11.1
             │   │   │   │       │   └── rgb011-check v0.1.0
             │   │   │   │       ├── rgb-psbt-utils v0.11.1
             │   │   │   │       │   ├── rgb-api v0.11.1 (*)
             │   │   │   │       │   └── rgb011-check v0.1.0 (*)
             │   │   │   │       ├── rgb-schemas v0.11.1
             │   │   │   │       │   └── rgb011-check v0.1.0 (*)
             │   │   │   │       └── rgb011-check v0.1.0 (*)
             │   │   │   ├── rgb-ops v0.11.1 (*)
             │   │   │   └── rgb011-check v0.1.0 (*)
             │   │   ├── rgb-ops v0.11.1 (*)
             │   │   └── rgb-schemas v0.11.1 (*)
             │   ├── rgb-api v0.11.1 (*)
             │   ├── rgb-ascii-armor v1.0.4
             │   │   ├── rgb-aluvm v0.11.1 (*)
             │   │   ├── rgb-ops v0.11.1 (*)
             │   │   └── rgb-strict-types v1.0.4
             │   │       ├── rgb-aluvm v0.11.1 (*)
             │   │       ├── rgb-api v0.11.1 (*)
             │   │       ├── rgb-consensus v0.11.1 (*)
             │   │       ├── rgb-invoicing v0.11.1 (*)
             │   │       ├── rgb-ops v0.11.1 (*)
             │   │       ├── rgb-schemas v0.11.1 (*)
             │   │       └── rgb011-check v0.1.0 (*)
             │   ├── rgb-consensus v0.11.1 (*)
             │   ├── rgb-invoicing v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-strict-types v1.0.4 (*)
             ├── nonasync v0.1.3
             │   ├── rgb-api v0.11.1 (*)
             │   └── rgb-ops v0.11.1 (*)
             ├── rgb-aluvm v0.11.1 (*)
             ├── rgb-api v0.11.1 (*)
             ├── rgb-ascii-armor v1.0.4 (*)
             ├── rgb-consensus v0.11.1 (*)
             ├── rgb-invoicing v0.11.1 (*)
             ├── rgb-ops v0.11.1 (*)
             ├── rgb-psbt-utils v0.11.1 (*)
             ├── rgb-schemas v0.11.1 (*)
             ├── rgb-strict-encoding v1.0.4
             │   ├── rgb-aluvm v0.11.1 (*)
             │   ├── rgb-ascii-armor v1.0.4 (*)
             │   ├── rgb-consensus v0.11.1 (*)
             │   ├── rgb-invoicing v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   ├── rgb-psbt-utils v0.11.1 (*)
             │   ├── rgb-strict-types v1.0.4 (*)
             │   └── rgb011-check v0.1.0 (*)
             ├── rgb-strict-types v1.0.4 (*)
             └── rgb011-check v0.1.0 (*)
   ├ rand_chacha v0.9.0
     └── rand v0.9.5
         ├── rgb-consensus v0.11.1
         │   ├── rgb-invoicing v0.11.1
         │   │   └── rgb-ops v0.11.1
         │   │       ├── rgb-api v0.11.1
         │   │       │   └── rgb011-check v0.1.0
         │   │       ├── rgb-psbt-utils v0.11.1
         │   │       │   ├── rgb-api v0.11.1 (*)
         │   │       │   └── rgb011-check v0.1.0 (*)
         │   │       ├── rgb-schemas v0.11.1
         │   │       │   └── rgb011-check v0.1.0 (*)
         │   │       └── rgb011-check v0.1.0 (*)
         │   ├── rgb-ops v0.11.1 (*)
         │   └── rgb011-check v0.1.0 (*)
         ├── rgb-ops v0.11.1 (*)
         └── secp256k1 v0.33.1
             └── rgb-consensus v0.11.1 (*)

warning[duplicate]: found 2 duplicate entries for crate 'rand_core'
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:83:1
   │  
83 │ ╭ rand_core 0.6.4 registry+https://github.com/rust-lang/crates.io-index
84 │ │ rand_core 0.9.5 registry+https://github.com/rust-lang/crates.io-index
   │ ╰─────────────────────────────────────────────────────────────────────┘ lock entries
   │  
   ├ rand_core v0.6.4
     ├── rand v0.8.8
     │   └── amplify v4.8.1
     │       ├── baid64 v0.4.1
     │       │   ├── rgb-aluvm v0.11.1
     │       │   │   ├── rgb-consensus v0.11.1
     │       │   │   │   ├── rgb-invoicing v0.11.1
     │       │   │   │   │   └── rgb-ops v0.11.1
     │       │   │   │   │       ├── rgb-api v0.11.1
     │       │   │   │   │       │   └── rgb011-check v0.1.0
     │       │   │   │   │       ├── rgb-psbt-utils v0.11.1
     │       │   │   │   │       │   ├── rgb-api v0.11.1 (*)
     │       │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │       │   │   │   │       ├── rgb-schemas v0.11.1
     │       │   │   │   │       │   └── rgb011-check v0.1.0 (*)
     │       │   │   │   │       └── rgb011-check v0.1.0 (*)
     │       │   │   │   ├── rgb-ops v0.11.1 (*)
     │       │   │   │   └── rgb011-check v0.1.0 (*)
     │       │   │   ├── rgb-ops v0.11.1 (*)
     │       │   │   └── rgb-schemas v0.11.1 (*)
     │       │   ├── rgb-api v0.11.1 (*)
     │       │   ├── rgb-ascii-armor v1.0.4
     │       │   │   ├── rgb-aluvm v0.11.1 (*)
     │       │   │   ├── rgb-ops v0.11.1 (*)
     │       │   │   └── rgb-strict-types v1.0.4
     │       │   │       ├── rgb-aluvm v0.11.1 (*)
     │       │   │       ├── rgb-api v0.11.1 (*)
     │       │   │       ├── rgb-consensus v0.11.1 (*)
     │       │   │       ├── rgb-invoicing v0.11.1 (*)
     │       │   │       ├── rgb-ops v0.11.1 (*)
     │       │   │       ├── rgb-schemas v0.11.1 (*)
     │       │   │       └── rgb011-check v0.1.0 (*)
     │       │   ├── rgb-consensus v0.11.1 (*)
     │       │   ├── rgb-invoicing v0.11.1 (*)
     │       │   ├── rgb-ops v0.11.1 (*)
     │       │   └── rgb-strict-types v1.0.4 (*)
     │       ├── nonasync v0.1.3
     │       │   ├── rgb-api v0.11.1 (*)
     │       │   └── rgb-ops v0.11.1 (*)
     │       ├── rgb-aluvm v0.11.1 (*)
     │       ├── rgb-api v0.11.1 (*)
     │       ├── rgb-ascii-armor v1.0.4 (*)
     │       ├── rgb-consensus v0.11.1 (*)
     │       ├── rgb-invoicing v0.11.1 (*)
     │       ├── rgb-ops v0.11.1 (*)
     │       ├── rgb-psbt-utils v0.11.1 (*)
     │       ├── rgb-schemas v0.11.1 (*)
     │       ├── rgb-strict-encoding v1.0.4
     │       │   ├── rgb-aluvm v0.11.1 (*)
     │       │   ├── rgb-ascii-armor v1.0.4 (*)
     │       │   ├── rgb-consensus v0.11.1 (*)
     │       │   ├── rgb-invoicing v0.11.1 (*)
     │       │   ├── rgb-ops v0.11.1 (*)
     │       │   ├── rgb-psbt-utils v0.11.1 (*)
     │       │   ├── rgb-strict-types v1.0.4 (*)
     │       │   └── rgb011-check v0.1.0 (*)
     │       ├── rgb-strict-types v1.0.4 (*)
     │       └── rgb011-check v0.1.0 (*)
     └── rand_chacha v0.3.1
         └── rand v0.8.8 (*)
   ├ rand_core v0.9.5
     ├── rand v0.9.5
     │   ├── rgb-consensus v0.11.1
     │   │   ├── rgb-invoicing v0.11.1
     │   │   │   └── rgb-ops v0.11.1
     │   │   │       ├── rgb-api v0.11.1
     │   │   │       │   └── rgb011-check v0.1.0
     │   │   │       ├── rgb-psbt-utils v0.11.1
     │   │   │       │   ├── rgb-api v0.11.1 (*)
     │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │   │       ├── rgb-schemas v0.11.1
     │   │   │       │   └── rgb011-check v0.1.0 (*)
     │   │   │       └── rgb011-check v0.1.0 (*)
     │   │   ├── rgb-ops v0.11.1 (*)
     │   │   └── rgb011-check v0.1.0 (*)
     │   ├── rgb-ops v0.11.1 (*)
     │   └── secp256k1 v0.33.1
     │       └── rgb-consensus v0.11.1 (*)
     └── rand_chacha v0.9.0
         └── rand v0.9.5 (*)

warning[duplicate]: found 2 duplicate entries for crate 'secp256k1'
    ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:102:1
    │  
102 │ ╭ secp256k1 0.29.1 registry+https://github.com/rust-lang/crates.io-index
103 │ │ secp256k1 0.33.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰──────────────────────────────────────────────────────────────────────┘ lock entries
    │  
    ├ secp256k1 v0.29.1
      ├── bitcoin v0.32.102
      │   ├── rgb-consensus v0.11.1
      │   │   ├── rgb-invoicing v0.11.1
      │   │   │   └── rgb-ops v0.11.1
      │   │   │       ├── rgb-api v0.11.1
      │   │   │       │   └── rgb011-check v0.1.0
      │   │   │       ├── rgb-psbt-utils v0.11.1
      │   │   │       │   ├── rgb-api v0.11.1 (*)
      │   │   │       │   └── rgb011-check v0.1.0 (*)
      │   │   │       ├── rgb-schemas v0.11.1
      │   │   │       │   └── rgb011-check v0.1.0 (*)
      │   │   │       └── rgb011-check v0.1.0 (*)
      │   │   ├── rgb-ops v0.11.1 (*)
      │   │   └── rgb011-check v0.1.0 (*)
      │   └── rgb-strict-encoding v1.0.4
      │       ├── rgb-aluvm v0.11.1
      │       │   ├── rgb-consensus v0.11.1 (*)
      │       │   ├── rgb-ops v0.11.1 (*)
      │       │   └── rgb-schemas v0.11.1 (*)
      │       ├── rgb-ascii-armor v1.0.4
      │       │   ├── rgb-aluvm v0.11.1 (*)
      │       │   ├── rgb-ops v0.11.1 (*)
      │       │   └── rgb-strict-types v1.0.4
      │       │       ├── rgb-aluvm v0.11.1 (*)
      │       │       ├── rgb-api v0.11.1 (*)
      │       │       ├── rgb-consensus v0.11.1 (*)
      │       │       ├── rgb-invoicing v0.11.1 (*)
      │       │       ├── rgb-ops v0.11.1 (*)
      │       │       ├── rgb-schemas v0.11.1 (*)
      │       │       └── rgb011-check v0.1.0 (*)
      │       ├── rgb-consensus v0.11.1 (*)
      │       ├── rgb-invoicing v0.11.1 (*)
      │       ├── rgb-ops v0.11.1 (*)
      │       ├── rgb-psbt-utils v0.11.1 (*)
      │       ├── rgb-strict-types v1.0.4 (*)
      │       └── rgb011-check v0.1.0 (*)
      └── o2a-demo-core v0.1.0
          └── rgb011-check v0.1.0 (*)
    ├ secp256k1 v0.33.1
      └── rgb-consensus v0.11.1
          ├── rgb-invoicing v0.11.1
          │   └── rgb-ops v0.11.1
          │       ├── rgb-api v0.11.1
          │       │   └── rgb011-check v0.1.0
          │       ├── rgb-psbt-utils v0.11.1
          │       │   ├── rgb-api v0.11.1 (*)
          │       │   └── rgb011-check v0.1.0 (*)
          │       ├── rgb-schemas v0.11.1
          │       │   └── rgb011-check v0.1.0 (*)
          │       └── rgb011-check v0.1.0 (*)
          ├── rgb-ops v0.11.1 (*)
          └── rgb011-check v0.1.0 (*)

warning[duplicate]: found 2 duplicate entries for crate 'secp256k1-sys'
    ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:104:1
    │  
104 │ ╭ secp256k1-sys 0.10.1 registry+https://github.com/rust-lang/crates.io-index
105 │ │ secp256k1-sys 0.14.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰──────────────────────────────────────────────────────────────────────────┘ lock entries
    │  
    ├ secp256k1-sys v0.10.1
      └── secp256k1 v0.29.1
          ├── bitcoin v0.32.102
          │   ├── rgb-consensus v0.11.1
          │   │   ├── rgb-invoicing v0.11.1
          │   │   │   └── rgb-ops v0.11.1
          │   │   │       ├── rgb-api v0.11.1
          │   │   │       │   └── rgb011-check v0.1.0
          │   │   │       ├── rgb-psbt-utils v0.11.1
          │   │   │       │   ├── rgb-api v0.11.1 (*)
          │   │   │       │   └── rgb011-check v0.1.0 (*)
          │   │   │       ├── rgb-schemas v0.11.1
          │   │   │       │   └── rgb011-check v0.1.0 (*)
          │   │   │       └── rgb011-check v0.1.0 (*)
          │   │   ├── rgb-ops v0.11.1 (*)
          │   │   └── rgb011-check v0.1.0 (*)
          │   └── rgb-strict-encoding v1.0.4
          │       ├── rgb-aluvm v0.11.1
          │       │   ├── rgb-consensus v0.11.1 (*)
          │       │   ├── rgb-ops v0.11.1 (*)
          │       │   └── rgb-schemas v0.11.1 (*)
          │       ├── rgb-ascii-armor v1.0.4
          │       │   ├── rgb-aluvm v0.11.1 (*)
          │       │   ├── rgb-ops v0.11.1 (*)
          │       │   └── rgb-strict-types v1.0.4
          │       │       ├── rgb-aluvm v0.11.1 (*)
          │       │       ├── rgb-api v0.11.1 (*)
          │       │       ├── rgb-consensus v0.11.1 (*)
          │       │       ├── rgb-invoicing v0.11.1 (*)
          │       │       ├── rgb-ops v0.11.1 (*)
          │       │       ├── rgb-schemas v0.11.1 (*)
          │       │       └── rgb011-check v0.1.0 (*)
          │       ├── rgb-consensus v0.11.1 (*)
          │       ├── rgb-invoicing v0.11.1 (*)
          │       ├── rgb-ops v0.11.1 (*)
          │       ├── rgb-psbt-utils v0.11.1 (*)
          │       ├── rgb-strict-types v1.0.4 (*)
          │       └── rgb011-check v0.1.0 (*)
          └── o2a-demo-core v0.1.0
              └── rgb011-check v0.1.0 (*)
    ├ secp256k1-sys v0.14.1
      └── secp256k1 v0.33.1
          └── rgb-consensus v0.11.1
              ├── rgb-invoicing v0.11.1
              │   └── rgb-ops v0.11.1
              │       ├── rgb-api v0.11.1
              │       │   └── rgb011-check v0.1.0
              │       ├── rgb-psbt-utils v0.11.1
              │       │   ├── rgb-api v0.11.1 (*)
              │       │   └── rgb011-check v0.1.0 (*)
              │       ├── rgb-schemas v0.11.1
              │       │   └── rgb011-check v0.1.0 (*)
              │       └── rgb011-check v0.1.0 (*)
              ├── rgb-ops v0.11.1 (*)
              └── rgb011-check v0.1.0 (*)

warning[duplicate]: found 2 duplicate entries for crate 'sha2'
    ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:110:1
    │  
110 │ ╭ sha2 0.10.9 registry+https://github.com/rust-lang/crates.io-index
111 │ │ sha2 0.11.0 registry+https://github.com/rust-lang/crates.io-index
    │ ╰─────────────────────────────────────────────────────────────────┘ lock entries
    │  
    ├ sha2 v0.10.9
      └── baid64 v0.4.1
          ├── rgb-aluvm v0.11.1
          │   ├── rgb-consensus v0.11.1
          │   │   ├── rgb-invoicing v0.11.1
          │   │   │   └── rgb-ops v0.11.1
          │   │   │       ├── rgb-api v0.11.1
          │   │   │       │   └── rgb011-check v0.1.0
          │   │   │       ├── rgb-psbt-utils v0.11.1
          │   │   │       │   ├── rgb-api v0.11.1 (*)
          │   │   │       │   └── rgb011-check v0.1.0 (*)
          │   │   │       ├── rgb-schemas v0.11.1
          │   │   │       │   └── rgb011-check v0.1.0 (*)
          │   │   │       └── rgb011-check v0.1.0 (*)
          │   │   ├── rgb-ops v0.11.1 (*)
          │   │   └── rgb011-check v0.1.0 (*)
          │   ├── rgb-ops v0.11.1 (*)
          │   └── rgb-schemas v0.11.1 (*)
          ├── rgb-api v0.11.1 (*)
          ├── rgb-ascii-armor v1.0.4
          │   ├── rgb-aluvm v0.11.1 (*)
          │   ├── rgb-ops v0.11.1 (*)
          │   └── rgb-strict-types v1.0.4
          │       ├── rgb-aluvm v0.11.1 (*)
          │       ├── rgb-api v0.11.1 (*)
          │       ├── rgb-consensus v0.11.1 (*)
          │       ├── rgb-invoicing v0.11.1 (*)
          │       ├── rgb-ops v0.11.1 (*)
          │       ├── rgb-schemas v0.11.1 (*)
          │       └── rgb011-check v0.1.0 (*)
          ├── rgb-consensus v0.11.1 (*)
          ├── rgb-invoicing v0.11.1 (*)
          ├── rgb-ops v0.11.1 (*)
          └── rgb-strict-types v1.0.4 (*)
    ├ sha2 v0.11.0
      ├── rgb-aluvm v0.11.1
      │   ├── rgb-consensus v0.11.1
      │   │   ├── rgb-invoicing v0.11.1
      │   │   │   └── rgb-ops v0.11.1
      │   │   │       ├── rgb-api v0.11.1
      │   │   │       │   └── rgb011-check v0.1.0
      │   │   │       ├── rgb-psbt-utils v0.11.1
      │   │   │       │   ├── rgb-api v0.11.1 (*)
      │   │   │       │   └── rgb011-check v0.1.0 (*)
      │   │   │       ├── rgb-schemas v0.11.1
      │   │   │       │   └── rgb011-check v0.1.0 (*)
      │   │   │       └── rgb011-check v0.1.0 (*)
      │   │   ├── rgb-ops v0.11.1 (*)
      │   │   └── rgb011-check v0.1.0 (*)
      │   ├── rgb-ops v0.11.1 (*)
      │   └── rgb-schemas v0.11.1 (*)
      ├── rgb-ascii-armor v1.0.4
      │   ├── rgb-aluvm v0.11.1 (*)
      │   ├── rgb-ops v0.11.1 (*)
      │   └── rgb-strict-types v1.0.4
      │       ├── rgb-aluvm v0.11.1 (*)
      │       ├── rgb-api v0.11.1 (*)
      │       ├── rgb-consensus v0.11.1 (*)
      │       ├── rgb-invoicing v0.11.1 (*)
      │       ├── rgb-ops v0.11.1 (*)
      │       ├── rgb-schemas v0.11.1 (*)
      │       └── rgb011-check v0.1.0 (*)
      ├── rgb-consensus v0.11.1 (*)
      └── rgb-strict-types v1.0.4 (*)

warning[duplicate]: found 3 duplicate entries for crate 'syn'
    ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/Cargo.lock:114:1
    │  
114 │ ╭ syn 1.0.109 registry+https://github.com/rust-lang/crates.io-index
115 │ │ syn 2.0.119 registry+https://github.com/rust-lang/crates.io-index
116 │ │ syn 3.0.6 registry+https://github.com/rust-lang/crates.io-index
    │ ╰───────────────────────────────────────────────────────────────┘ lock entries
    │  
    ├ syn v1.0.109
      ├── amplify_derive v4.0.1
      │   └── amplify v4.8.1
      │       ├── baid64 v0.4.1
      │       │   ├── rgb-aluvm v0.11.1
      │       │   │   ├── rgb-consensus v0.11.1
      │       │   │   │   ├── rgb-invoicing v0.11.1
      │       │   │   │   │   └── rgb-ops v0.11.1
      │       │   │   │   │       ├── rgb-api v0.11.1
      │       │   │   │   │       │   └── rgb011-check v0.1.0
      │       │   │   │   │       ├── rgb-psbt-utils v0.11.1
      │       │   │   │   │       │   ├── rgb-api v0.11.1 (*)
      │       │   │   │   │       │   └── rgb011-check v0.1.0 (*)
      │       │   │   │   │       ├── rgb-schemas v0.11.1
      │       │   │   │   │       │   └── rgb011-check v0.1.0 (*)
      │       │   │   │   │       └── rgb011-check v0.1.0 (*)
      │       │   │   │   ├── rgb-ops v0.11.1 (*)
      │       │   │   │   └── rgb011-check v0.1.0 (*)
      │       │   │   ├── rgb-ops v0.11.1 (*)
      │       │   │   └── rgb-schemas v0.11.1 (*)
      │       │   ├── rgb-api v0.11.1 (*)
      │       │   ├── rgb-ascii-armor v1.0.4
      │       │   │   ├── rgb-aluvm v0.11.1 (*)
      │       │   │   ├── rgb-ops v0.11.1 (*)
      │       │   │   └── rgb-strict-types v1.0.4
      │       │   │       ├── rgb-aluvm v0.11.1 (*)
      │       │   │       ├── rgb-api v0.11.1 (*)
      │       │   │       ├── rgb-consensus v0.11.1 (*)
      │       │   │       ├── rgb-invoicing v0.11.1 (*)
      │       │   │       ├── rgb-ops v0.11.1 (*)
      │       │   │       ├── rgb-schemas v0.11.1 (*)
      │       │   │       └── rgb011-check v0.1.0 (*)
      │       │   ├── rgb-consensus v0.11.1 (*)
      │       │   ├── rgb-invoicing v0.11.1 (*)
      │       │   ├── rgb-ops v0.11.1 (*)
      │       │   └── rgb-strict-types v1.0.4 (*)
      │       ├── nonasync v0.1.3
      │       │   ├── rgb-api v0.11.1 (*)
      │       │   └── rgb-ops v0.11.1 (*)
      │       ├── rgb-aluvm v0.11.1 (*)
      │       ├── rgb-api v0.11.1 (*)
      │       ├── rgb-ascii-armor v1.0.4 (*)
      │       ├── rgb-consensus v0.11.1 (*)
      │       ├── rgb-invoicing v0.11.1 (*)
      │       ├── rgb-ops v0.11.1 (*)
      │       ├── rgb-psbt-utils v0.11.1 (*)
      │       ├── rgb-schemas v0.11.1 (*)
      │       ├── rgb-strict-encoding v1.0.4
      │       │   ├── rgb-aluvm v0.11.1 (*)
      │       │   ├── rgb-ascii-armor v1.0.4 (*)
      │       │   ├── rgb-consensus v0.11.1 (*)
      │       │   ├── rgb-invoicing v0.11.1 (*)
      │       │   ├── rgb-ops v0.11.1 (*)
      │       │   ├── rgb-psbt-utils v0.11.1 (*)
      │       │   ├── rgb-strict-types v1.0.4 (*)
      │       │   └── rgb011-check v0.1.0 (*)
      │       ├── rgb-strict-types v1.0.4 (*)
      │       └── rgb011-check v0.1.0 (*)
      ├── amplify_syn v2.0.1
      │   ├── amplify v4.8.1 (*)
      │   ├── amplify_derive v4.0.1 (*)
      │   └── rgb-strict-encoding-derive v1.0.4
      │       └── rgb-strict-encoding v1.0.4 (*)
      └── rgb-strict-encoding-derive v1.0.4 (*)
    ├ syn v2.0.119
      ├── thiserror-impl v1.0.69
      │   └── thiserror v1.0.69
      │       └── base85 v2.0.0
      │           ├── rgb-ascii-armor v1.0.4
      │           │   ├── rgb-aluvm v0.11.1
      │           │   │   ├── rgb-consensus v0.11.1
      │           │   │   │   ├── rgb-invoicing v0.11.1
      │           │   │   │   │   └── rgb-ops v0.11.1
      │           │   │   │   │       ├── rgb-api v0.11.1
      │           │   │   │   │       │   └── rgb011-check v0.1.0
      │           │   │   │   │       ├── rgb-psbt-utils v0.11.1
      │           │   │   │   │       │   ├── rgb-api v0.11.1 (*)
      │           │   │   │   │       │   └── rgb011-check v0.1.0 (*)
      │           │   │   │   │       ├── rgb-schemas v0.11.1
      │           │   │   │   │       │   └── rgb011-check v0.1.0 (*)
      │           │   │   │   │       └── rgb011-check v0.1.0 (*)
      │           │   │   │   ├── rgb-ops v0.11.1 (*)
      │           │   │   │   └── rgb011-check v0.1.0 (*)
      │           │   │   ├── rgb-ops v0.11.1 (*)
      │           │   │   └── rgb-schemas v0.11.1 (*)
      │           │   ├── rgb-ops v0.11.1 (*)
      │           │   └── rgb-strict-types v1.0.4
      │           │       ├── rgb-aluvm v0.11.1 (*)
      │           │       ├── rgb-api v0.11.1 (*)
      │           │       ├── rgb-consensus v0.11.1 (*)
      │           │       ├── rgb-invoicing v0.11.1 (*)
      │           │       ├── rgb-ops v0.11.1 (*)
      │           │       ├── rgb-schemas v0.11.1 (*)
      │           │       └── rgb011-check v0.1.0 (*)
      │           └── rgb-consensus v0.11.1 (*)
      ├── windows-implement v0.60.2
      │   └── windows-core v0.62.2
      │       └── iana-time-zone v0.1.65
      │           └── chrono v0.4.45
      │               ├── rgb-api v0.11.1 (*)
      │               ├── rgb-consensus v0.11.1 (*)
      │               └── rgb-ops v0.11.1 (*)
      ├── windows-interface v0.59.3
      │   └── windows-core v0.62.2 (*)
      └── zerocopy-derive v0.8.59
          └── zerocopy v0.8.59
              ├── half v2.7.1
              │   └── rgb-aluvm v0.11.1 (*)
              └── ppv-lite86 v0.2.21
                  ├── rand_chacha v0.3.1
                  │   └── rand v0.8.8
                  │       └── amplify v4.8.1
                  │           ├── baid64 v0.4.1
                  │           │   ├── rgb-aluvm v0.11.1 (*)
                  │           │   ├── rgb-api v0.11.1 (*)
                  │           │   ├── rgb-ascii-armor v1.0.4 (*)
                  │           │   ├── rgb-consensus v0.11.1 (*)
                  │           │   ├── rgb-invoicing v0.11.1 (*)
                  │           │   ├── rgb-ops v0.11.1 (*)
                  │           │   └── rgb-strict-types v1.0.4 (*)
                  │           ├── nonasync v0.1.3
                  │           │   ├── rgb-api v0.11.1 (*)
                  │           │   └── rgb-ops v0.11.1 (*)
                  │           ├── rgb-aluvm v0.11.1 (*)
                  │           ├── rgb-api v0.11.1 (*)
                  │           ├── rgb-ascii-armor v1.0.4 (*)
                  │           ├── rgb-consensus v0.11.1 (*)
                  │           ├── rgb-invoicing v0.11.1 (*)
                  │           ├── rgb-ops v0.11.1 (*)
                  │           ├── rgb-psbt-utils v0.11.1 (*)
                  │           ├── rgb-schemas v0.11.1 (*)
                  │           ├── rgb-strict-encoding v1.0.4
                  │           │   ├── rgb-aluvm v0.11.1 (*)
                  │           │   ├── rgb-ascii-armor v1.0.4 (*)
                  │           │   ├── rgb-consensus v0.11.1 (*)
                  │           │   ├── rgb-invoicing v0.11.1 (*)
                  │           │   ├── rgb-ops v0.11.1 (*)
                  │           │   ├── rgb-psbt-utils v0.11.1 (*)
                  │           │   ├── rgb-strict-types v1.0.4 (*)
                  │           │   └── rgb011-check v0.1.0 (*)
                  │           ├── rgb-strict-types v1.0.4 (*)
                  │           └── rgb011-check v0.1.0 (*)
                  └── rand_chacha v0.9.0
                      └── rand v0.9.5
                          ├── rgb-consensus v0.11.1 (*)
                          ├── rgb-ops v0.11.1 (*)
                          └── secp256k1 v0.33.1
                              └── rgb-consensus v0.11.1 (*)
    ├ syn v3.0.6
      ├── ref-cast-impl v1.0.27
      │   └── ref-cast v1.0.27
      │       └── fluent-uri v0.3.2
      │           └── rgb-invoicing v0.11.1
      │               └── rgb-ops v0.11.1
      │                   ├── rgb-api v0.11.1
      │                   │   └── rgb011-check v0.1.0
      │                   ├── rgb-psbt-utils v0.11.1
      │                   │   ├── rgb-api v0.11.1 (*)
      │                   │   └── rgb011-check v0.1.0 (*)
      │                   ├── rgb-schemas v0.11.1
      │                   │   └── rgb011-check v0.1.0 (*)
      │                   └── rgb011-check v0.1.0 (*)
      ├── serde_derive v1.0.229
      │   ├── petgraph v0.8.3
      │   │   └── daggy v0.9.0
      │   │       └── rgb-consensus v0.11.1
      │   │           ├── rgb-invoicing v0.11.1 (*)
      │   │           ├── rgb-ops v0.11.1 (*)
      │   │           └── rgb011-check v0.1.0 (*)
      │   └── serde v1.0.229
      │       ├── bitcoin v0.32.102
      │       │   ├── rgb-consensus v0.11.1 (*)
      │       │   └── rgb-strict-encoding v1.0.4
      │       │       ├── rgb-aluvm v0.11.1
      │       │       │   ├── rgb-consensus v0.11.1 (*)
      │       │       │   ├── rgb-ops v0.11.1 (*)
      │       │       │   └── rgb-schemas v0.11.1 (*)
      │       │       ├── rgb-ascii-armor v1.0.4
      │       │       │   ├── rgb-aluvm v0.11.1 (*)
      │       │       │   ├── rgb-ops v0.11.1 (*)
      │       │       │   └── rgb-strict-types v1.0.4
      │       │       │       ├── rgb-aluvm v0.11.1 (*)
      │       │       │       ├── rgb-api v0.11.1 (*)
      │       │       │       ├── rgb-consensus v0.11.1 (*)
      │       │       │       ├── rgb-invoicing v0.11.1 (*)
      │       │       │       ├── rgb-ops v0.11.1 (*)
      │       │       │       ├── rgb-schemas v0.11.1 (*)
      │       │       │       └── rgb011-check v0.1.0 (*)
      │       │       ├── rgb-consensus v0.11.1 (*)
      │       │       ├── rgb-invoicing v0.11.1 (*)
      │       │       ├── rgb-ops v0.11.1 (*)
      │       │       ├── rgb-psbt-utils v0.11.1 (*)
      │       │       ├── rgb-strict-types v1.0.4 (*)
      │       │       └── rgb011-check v0.1.0 (*)
      │       ├── bitcoin-units v0.1.101
      │       │   └── bitcoin v0.32.102 (*)
      │       ├── bitcoin_hashes v0.14.101
      │       │   ├── base58ck v0.1.101
      │       │   │   └── bitcoin v0.32.102 (*)
      │       │   ├── bitcoin v0.32.102 (*)
      │       │   ├── o2a-demo-core v0.1.0
      │       │   │   └── rgb011-check v0.1.0 (*)
      │       │   └── secp256k1 v0.29.1
      │       │       ├── bitcoin v0.32.102 (*)
      │       │       └── o2a-demo-core v0.1.0 (*)
      │       ├── daggy v0.9.0 (*)
      │       ├── petgraph v0.8.3 (*)
      │       ├── rgb-psbt-utils v0.11.1 (*)
      │       ├── secp256k1 v0.29.1 (*)
      │       └── serde_json v1.0.140
      │           └── rgb011-check v0.1.0 (*)
      └── wasm-bindgen-macro-support v0.2.129
          └── wasm-bindgen-macro v0.2.129
              └── wasm-bindgen v0.2.129
                  ├── amplify v4.8.1
                  │   ├── baid64 v0.4.1
                  │   │   ├── rgb-aluvm v0.11.1 (*)
                  │   │   ├── rgb-api v0.11.1 (*)
                  │   │   ├── rgb-ascii-armor v1.0.4 (*)
                  │   │   ├── rgb-consensus v0.11.1 (*)
                  │   │   ├── rgb-invoicing v0.11.1 (*)
                  │   │   ├── rgb-ops v0.11.1 (*)
                  │   │   └── rgb-strict-types v1.0.4 (*)
                  │   ├── nonasync v0.1.3
                  │   │   ├── rgb-api v0.11.1 (*)
                  │   │   └── rgb-ops v0.11.1 (*)
                  │   ├── rgb-aluvm v0.11.1 (*)
                  │   ├── rgb-api v0.11.1 (*)
                  │   ├── rgb-ascii-armor v1.0.4 (*)
                  │   ├── rgb-consensus v0.11.1 (*)
                  │   ├── rgb-invoicing v0.11.1 (*)
                  │   ├── rgb-ops v0.11.1 (*)
                  │   ├── rgb-psbt-utils v0.11.1 (*)
                  │   ├── rgb-schemas v0.11.1 (*)
                  │   ├── rgb-strict-encoding v1.0.4 (*)
                  │   ├── rgb-strict-types v1.0.4 (*)
                  │   └── rgb011-check v0.1.0 (*)
                  ├── amplify_apfloat v0.3.1
                  │   └── amplify v4.8.1 (*)
                  ├── amplify_num v0.5.4
                  │   ├── amplify v4.8.1 (*)
                  │   └── amplify_apfloat v0.3.1 (*)
                  ├── chrono v0.4.45
                  │   ├── rgb-api v0.11.1 (*)
                  │   ├── rgb-consensus v0.11.1 (*)
                  │   └── rgb-ops v0.11.1 (*)
                  ├── getrandom v0.2.17
                  │   ├── rand_core v0.6.4
                  │   │   ├── rand v0.8.8
                  │   │   │   └── amplify v4.8.1 (*)
                  │   │   └── rand_chacha v0.3.1
                  │   │       └── rand v0.8.8 (*)
                  │   └── rgb-consensus v0.11.1 (*)
                  ├── getrandom v0.3.4
                  │   ├── rand_core v0.9.5
                  │   │   ├── rand v0.9.5
                  │   │   │   ├── rgb-consensus v0.11.1 (*)
                  │   │   │   ├── rgb-ops v0.11.1 (*)
                  │   │   │   └── secp256k1 v0.33.1
                  │   │   │       └── rgb-consensus v0.11.1 (*)
                  │   │   └── rand_chacha v0.9.0
                  │   │       └── rand v0.9.5 (*)
                  │   ├── rgb-api v0.11.1 (*)
                  │   ├── rgb-consensus v0.11.1 (*)
                  │   ├── rgb-ops v0.11.1 (*)
                  │   └── rgb-psbt-utils v0.11.1 (*)
                  ├── iana-time-zone v0.1.65
                  │   └── chrono v0.4.45 (*)
                  ├── js-sys v0.3.106
                  │   ├── chrono v0.4.45 (*)
                  │   ├── getrandom v0.2.17 (*)
                  │   ├── getrandom v0.3.4 (*)
                  │   └── iana-time-zone v0.1.65 (*)
                  ├── rgb-aluvm v0.11.1 (*)
                  ├── rgb-api v0.11.1 (*)
                  ├── rgb-consensus v0.11.1 (*)
                  ├── rgb-ops v0.11.1 (*)
                  ├── rgb-psbt-utils v0.11.1 (*)
                  ├── rgb-strict-encoding v1.0.4 (*)
                  └── rgb-strict-types v1.0.4 (*)

advisories ok, bans ok, sources ok
EXIT:0
```

The license report exited 4. That exit does not block under D14:

```text
$ cargo deny check licenses
error[rejected]: failed to satisfy license requirements
   ┌─ /home/kestl/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hex_lit-0.1.1/Cargo.toml:22:12
   │
22 │ license = "MITNFA"
   │            ━━━━━━
   │            │
   │            rejected: license is not explicitly allowed
   │
   ├ MITNFA - MIT +no-false-attribs license:
   ├   - No additional metadata available for license
   ├ hex_lit v0.1.1
     └── bitcoin v0.32.102
         ├── rgb-consensus v0.11.1
         │   ├── rgb-invoicing v0.11.1
         │   │   └── rgb-ops v0.11.1
         │   │       ├── rgb-api v0.11.1
         │   │       │   └── rgb011-check v0.1.0
         │   │       ├── rgb-psbt-utils v0.11.1
         │   │       │   ├── rgb-api v0.11.1 (*)
         │   │       │   └── rgb011-check v0.1.0 (*)
         │   │       ├── rgb-schemas v0.11.1
         │   │       │   └── rgb011-check v0.1.0 (*)
         │   │       └── rgb011-check v0.1.0 (*)
         │   ├── rgb-ops v0.11.1 (*)
         │   └── rgb011-check v0.1.0 (*)
         └── rgb-strict-encoding v1.0.4
             ├── rgb-aluvm v0.11.1
             │   ├── rgb-consensus v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-schemas v0.11.1 (*)
             ├── rgb-ascii-armor v1.0.4
             │   ├── rgb-aluvm v0.11.1 (*)
             │   ├── rgb-ops v0.11.1 (*)
             │   └── rgb-strict-types v1.0.4
             │       ├── rgb-aluvm v0.11.1 (*)
             │       ├── rgb-api v0.11.1 (*)
             │       ├── rgb-consensus v0.11.1 (*)
             │       ├── rgb-invoicing v0.11.1 (*)
             │       ├── rgb-ops v0.11.1 (*)
             │       ├── rgb-schemas v0.11.1 (*)
             │       └── rgb011-check v0.1.0 (*)
             ├── rgb-consensus v0.11.1 (*)
             ├── rgb-invoicing v0.11.1 (*)
             ├── rgb-ops v0.11.1 (*)
             ├── rgb-psbt-utils v0.11.1 (*)
             ├── rgb-strict-types v1.0.4 (*)
             └── rgb011-check v0.1.0 (*)

warning[license-not-encountered]: license was not encountered
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/deny.toml:12:6
   │
12 │     "BSD-3-Clause",
   │      ━━━━━━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/deny.toml:14:6
   │
14 │     "CDLA-Permissive-2.0",
   │      ━━━━━━━━━━━━━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /media/kestl/andor/ffwd/wt-rgb011-spike/spikes/rgb-0.11.1/deny.toml:15:6
   │
15 │     "ISC",
   │      ━━━ unmatched license allowance

licenses FAILED
EXIT:4
```

The complete files are `evidence/regtest-rgb011-compat-2026-09-28-addendum/deny-advisories-bans-sources.txt` and `deny-licenses.txt`. The spike gate script `spikes/rgb-0.11.1/check-dependency-policy.sh` exited 0 and printed `cargo deny licenses report-only: exit 4`.

The NO-GO section above remains the record of the 2026-09-24 gate. Under D14 the compatibility result is GO. No adapter port is in this revision. That work begins after ADR-0010 is accepted.
