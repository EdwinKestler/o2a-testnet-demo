Disposable demo-lineage evidence. This is not a Phase 0 gate closure. No normative seal script is adopted.

# Regtest smoke: custom tapscript seals on RGB 0.12 RC3

Regtest only. Keys come from the demo's published unsafe seed on the path `m/9999'/1'/role'/index'`. That path is smoke-only and non-normative. It is not BIP86 and it is not an O2A identity role path. Signet and mainnet were not used.

## Verdict

RGB 0.12 RC3 contract layer accepts custom-tapscript seals: YES

RGB wallet can track or sign them: NO

The contract layer assigned state to these outpoints, accepted spends through a single-key leaf and through the 2-of-3 recovery leaf, and two fresh validators agreed. The RGB wallet's descriptor does not list the outpoints and does not finalize a script-path witness for them.

## Results

| Test | Result | Evidence |
| --- | --- | --- |
| E1 External seal accepts this script | PASS | `issue-and-wallet.txt`, `verify-a-b.txt` |
| E2 Wallet does not track or select them | PASS | `issue-and-wallet.txt` |
| E3 `rgb_fill_csv` and `complete` on a foreign seal input | PASS | `anchor-a-b.txt` |
| E4 Script-path signing | PASS | `anchor-a-b.txt`, `variant-c.txt` |
| E5 Bitcoin enforces the recovery delay | PASS | `bip68.txt` |
| E6 Delay clock is the seal output's confirmation | PASS | `bip68.txt` |
| S6 Seal spent with no RGB commitment | RECORDED | `plain-and-s6.txt` |

S6 is not scored. The raw runtime and validator lines are copied below and in `plain-and-s6.txt`.

## Environment

- Demo branch base: `def57d7dc1d0fb438fb7a19291d9f4ea6ca5123d` on `main`.
- Spike branch: `spike/seal-tapscript-smoke`. Committed locally. Not pushed.
- Toolchain image: `rust:1.98.1-slim-bookworm` from `dev/Dockerfile.toolchain`.
- Bitcoin Core 31.1.0. Electrs 0.12.0. Host check: `./dev/check-host.sh` PASS. Docker context `default`, Docker 29.8.1.
- Compose project: `o2a-seal-smoke`.
- RGB-WG pin: `a1e6b41524131f6d6f183b2235fdaacb5c1abb31`.
- Applied witness-height patch: `f1e5a68992700ce208da81682ff092c0d5576e82` on `rgb-runtime` and `rgb-psbt`. See `PATCHES.md`. No new upstream patch was added.
- `cargo check --workspace --locked` finished in 0.20s, exit 0.
- `cargo audit --ignore RUSTSEC-2024-0436` scanned 191 crates, exit 0.
- `cargo deny check` exit 0: `advisories ok, bans ok, licenses ok, sources ok`. It warned that the allowed RGB-WG source was unused (the patch redirects those crates), that the Zlib allowance was unused, and that `getrandom` has duplicate versions. Raw output is `checks.txt`.
- `Cargo.lock` gained only the local `o2a-seal-smoke` package. Diff: `Cargo.lock.diff`.

The regtest subnet on this branch is `172.30.32.0/24`. Project `o2a-testnet-demo` was already exited and still held `172.30.30.0/24`, so the smoke project could not use that subnet. That project and its volumes were not changed. Signet stayed up. `o2a-phase0` was not touched. Volume list: `volumes.txt`.

## Seal script

Internal key is BIP341 NUMS H, `50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0`. There is no key-path spend.

Leaves use version `0xc0`. Controller leaves are `<xonly> OP_CHECKSIG`, sorted by key. The recovery leaf is Core 31.1's compilation of `and_v(v:multi_a(2,R1,R2,R3),older(10))`:

`OP_2 OP_NUMEQUALVERIFY OP_10 OP_CHECKSEQUENCEVERIFY`

Keys are sorted R1, R2, R3. There is no `OP_DROP`. The tree is controller leaves, then the recovery leaf, pairwise from the left. An odd last node is carried up. Seal B is that odd case.

| Seal | Shape | scriptPubKey |
| --- | --- | --- |
| A | C1 + recovery | `5120c1e07f0d505ad01c072b1f718af10baea7babd8c9f39317fe29f0cfc500debba` |
| B | C1 + C2 + recovery | `51206e99a38726c796b46e53ca51437c2f1cce4fab6a971f08c7d9faa1b8b2225787` |
| C | C2 + recovery | `5120f2489653394b1a911fd8a8b5a3983ba327d0414264d676d794bf6b86ca518954` |

Each one matches Bitcoin Core `getdescriptorinfo`, `deriveaddresses`, and `getaddressinfo` byte for byte. Addresses and checksums are in `core-descriptors.txt`.

## Identifiers

Funding transaction `46475f1eb02b18b99e9975dbfd833c6f403c3649ed4ef8b55f28541359e30878`, confirmed at height 102 in `3ad4fdaf34ea77e4b44d2c7221e066394f96ab0bf08146d1aa44af2333188c97`.

| Output | vout | amount |
| --- | --- | --- |
| Seal A | 0 | 100000 sats |
| RGB fee | 1 | 100000 sats |
| Seal B | 2 | 100000 sats |
| Seal C | 3 | 100000 sats |

Contract `contract:zf3wG3sS-CX225e8-ppsigBg-rOP4UrJ-XBGNp8e-Ab1HX5c`.

Genesis cell `th8kp_wbJjiz3DgRsk5WARn3tjbVNFaYmzhPgKNq3_U:0` on seal A. O2A digest `246b8b2c0f5377ddebdb285deb19da8f83a999b9119fd888f9e9d30519573164`. Object: `genesis.o2a`.

A to B anchor `db952ddece17f82905e8f562f1b62b3d877189403780efd7a8d18d6b16f08fc3`, mined at height 103 in `34e3ff674dc411be200ca604078af24c7418668c3ba543a0664af58f7888c163`. Cell `B8J4txlF~_1T3kMNREHeWoSjNvT_UK4ZvDyhPF6N75g:0` on seal B. Files: `rotation-a-b.rgb`, `rotation-a-b.o2a`.

Recovery anchor `425c4c517e8bd54492554f71c9b1b29d8a17474cd0c369e619dfb760abc7a619`, mined at height 113 in `3a6ea6bc1e701be6183cca405d88deaf47008f2d0ce4ee4216f675ec75c15dee`. Cell `Ei9WPd9b1ea9dpWn9KBbvjBnb1FLrAsPkQv6y3zajGo:0` on seal C. Files: `recovery-b-c.rgb`, `recovery-b-c.o2a`. This transition is mechanism-only.

Plain spend of seal C `af42d080ff79c22166e3bda37ca68789111180671646c517a86d3a054d45310d`, mined at height 114 in `766186f0b9e27ce191fcd1dfbcea6162a727040b6bff9ec6489862850a6f78ff`. No RGB commitment.

## Procedure notes

Genesis used `Assignment::new_internal` on outpoint A. "Internal" here is the assignment form. The outpoint was not a wallet UTXO. After issue, `runtime.wallet.utxos()` contained only the fee output. `construct_psbt` rejected A, B, and C with "is not known for the current wallet" and accepted the fee output.

`RgbDescr::new_unfunded` accepts only `Wpkh` and key-only `Tr`. Any other descriptor hits `unreachable!()` (`descriptors/src/lib.rs:259-263` in checkout `f1e5a68`). `key_only_unfunded` is `Tr::KeyOnly` (`descriptors/src/lib.rs:270-277`).

`RgbRuntime::complete` commits the deterministic bitcoin commitment and includes the bundle from previous outpoints (`src/runtime.rs:295-310`). It does not require the seal input to belong to the wallet. Both anchors completed. Witness status after one confirmation was `Mined(103)` and `Mined(113)`, the same heights Bitcoin Core reported.

Signing, in the required order:

1. `TestnetSigner::new_script_spent` produced one script-path signature for the C1 leaf, and one each for R1 and R3 on the recovery leaf. `psbt.finalize(descriptor)` finalized only the fee input. `variant_a_seal_finalized=false`. `TrKey::taproot_witness` returns no witness when a control block is present (`descriptors-0.12.0-rc.3/src/tr.rs:235-244`). The PSBT finalizer also reads `merkle_branch.first()` as if it were the leaf hash (`psbt-0.12.0-rc.3/src/data.rs:890-891`).
2. A manual BIP341 script-path sighash and BIP340 signature, with witness `[sig, leaf script, control block]`, was accepted by Core. The first `extract()` panicked at `psbt-0.12.0-rc.3/src/data.rs:793` because `final_script_sig` was missing. An empty scriptSig plus the witness extracted. `testmempoolaccept` returned `allowed=true` and `sendrawtransaction` returned the txid.
3. Optional. A version-0 PSBT round-tripped through Core 31.1. `decodepsbt` accepted it. `walletprocesspsbt` returned `complete=true` and a three-item script-path witness whose script and control block match the manual C2 spend. Import needed `active=false` because the descriptor is not ranged. The public form of the descriptor was rejected. Details and the two import errors are in `variant-c.txt`.

The recovery transaction uses version 2 and `nSequence=10` on seal B, with R1 and R3 signatures and an empty R2 item. At height 103, `sendrawtransaction` returned:

```text
error code: -26
error message:
non-BIP68-final
```

`testmempoolaccept` allowed the same transaction at height 111. It was broadcast at height 112 and mined in block 113. RGB then reported the cell on seal C with status `Mined(113)`. Two fresh validators printed the same bytes and `o2a_semantics=not_applicable`.

E6 heights:

| Clock | Height |
| --- | --- |
| B funding confirmation | 102 |
| S4 anchor that named B | 103 |
| First `testmempoolaccept` success | 111 |
| `sendrawtransaction` success | 112 |
| Block that included the recovery spend | 113 |
| Task quote "prior anchor height + delay_blocks" | 113 |
| BIP68 inclusion, funding height + 10 | 112 |

Bitcoin allowed the spend one block before the anchor-plus-10 quote. The relative lock is measured from confirmation of the output being spent.

The protocol files read at `b731c19ca9eb3ff36bd7ca6504cc967d7d1ea15f` on `spec/seal-key-role-and-script` already say the same thing: `not_before_height` is the confirmation height of the transaction that created the current seal output plus `delay_blocks` (`specs/rgb-identity-contract.md:75-77` and `:90-98`, `specs/canonical-encoding.md:314-316`). This smoke test did not edit that repository.

## S6, quoted

After seal C was spent and mined, with electrs at height 114:

```text
runtime_update=ok
owned name=identity cell=Ei9WPd9b1ea9dpWn9KBbvjBnb1FLrAsPkQv6y3zajGo:0 seal=46475f1eb02b18b99e9975dbfd833c6f403c3649ed4ef8b55f28541359e30878:3/00000000000000000000000000000000000000000000000000000000000000000000000000000000 status=Mined(113)
```

A fresh validator given only the recovery consignment, the O2A object, and electrs printed:

```text
electrum_height=114
rgb_contract_id=contract:zf3wG3sS-CX225e8-ppsigBg-rOP4UrJ-XBGNp8e-Ab1HX5c
rgb_current_cell=Ei9WPd9b1ea9dpWn9KBbvjBnb1FLrAsPkQv6y3zajGo:0
rgb_current_seal=46475f1eb02b18b99e9975dbfd833c6f403c3649ed4ef8b55f28541359e30878:3/00000000000000000000000000000000000000000000000000000000000000000000000000000000
bitcoin_order_spend=Mined(113)
o2a_semantics=not_applicable
```

No error string was printed.

## What sits outside RGB

O2A has to keep the seal script, the smoke-only keys, and the outpoint set. The RGB wallet will not do that. O2A has to build the PSBT: `witness_utxo`, `tap_internal_key`, `tap_merkle_root`, and `tap_leaf_script`. O2A has to produce the script-path witness and set an empty `final_script_sig`, because `extract()` requires it. The fee input can stay on the key-only RGB descriptor; that input did finalize. Recovery still has to put the empty unselected signature in the right stack position and set version 2 and `nSequence`.

## Candidate upstream needs

These are candidates only. Nothing in `o2a-protocol` was edited.

- `RgbDescr` would need a script-tree descriptor before a wallet could track these outputs. Today only `Wpkh` and key-only `Tr` construct.
- The PSBT finalizer would need to identify a tap leaf by its leaf hash. It currently uses the first merkle sibling.
- `extract()` panics unless `final_script_sig` is set, including on a taproot input whose scriptSig is empty.
- An external-seal coin selector would be new wallet behavior. `construct_psbt` rejects an outpoint the wallet does not know.
