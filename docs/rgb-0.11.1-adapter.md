# Maintained RGB adapter: rgb-protocol 0.11.1

**Status:** current demo adapter. ADR-0010 accepts rgb-protocol 0.11.1 with Opret.

The archived RGB-WG 0.12 program remains in [rgb-program-demo.md](rgb-program-demo.md). This file describes the 0.11.1 identity schema, the Opret spend, the workspace boundary, the C6 and C7 result that O2A enforces, and the commands that run it.

The adapter is disposable demo evidence. It does not enter the normative specification. It refuses mainnet.

## Identity schema

The schema is built in `spikes/rgb-0.11.1/check/src/schema.rs`. Validators stay unset. RGB checks the consignment shape. O2A checks the signed object.

The owned state is a Declarative right. The 32-byte value is global state, not the owned cell. The 0.12 program stored that digest in the owned identity cell. This schema does not.

| Item | Value |
| --- | --- |
| Type library | `O2AIdentity011` |
| Strict type | `O2aDigest([u8; 32])` |
| Global state | type `3101`, name `digest`, `Occurrences::Once` |
| Owned state | type `4101`, name `identity`, `OwnedStateSchema::Declarative`, default transition `8101` |
| Transitions | `8101` rotate, `8102` recover, `8103` revoke |
| Revoke outputs | none |
| Genesis validator | `None` |
| Transition validators | `None` |
| Schema name | `O2aIdentity` |
| Schema id | `rgb:sch:oqE1HKzG_NrzhV2M0tn~mkfJfic5ztF6iUr8s8YbdDE#ivan-robin-exotic` |

Genesis calls `add_global_state("digest", O2aDigest)` and `add_rights("identity", …)` on a blinded outpoint seal. The issue timestamp constant is `1759017600`. The compatibility bundle records this schema id.

The transition types are declared so a later program can use them. ADR-0009 keeps transitions off a frozen identity until the RGB stack, the program, and the carrier are final. The stage signs genesis and one `official_name` claim. It does not broadcast rotate, recover, or revoke.

## Opret flow

`spikes/rgb-0.11.1/check/src/spend.rs` function `commit_opret` is the spend used by the maintained adapter. Tapret on the O2A script-tree host is unsupported. Compatibility case C9 recorded `use of taproot script descriptors is not yet supported.` The accepted close method is Opret.

1. Build a version-2 transaction with locktime zero.
2. Put an empty `OP_RETURN` first. The script is the single byte `0x6a`.
3. Put the next seal output second, then the change output.
4. Input 0 spends the current seal. Its sequence is the value passed in.
5. Input 1 spends the fee output. Its sequence is `Sequence::MAX`.
6. Fill both PSBT inputs with `witness_utxo`. On the seal input, set the NUMS `tap_internal_key` and `tap_merkle_root`.
7. Call `set_rgb_close_method(CloseMethod::OpretFirst)`, `set_opret_host`, `set_as_unmodifiable`, `push_rgb_transition`, and `rgb_commit`.
8. Require the fascia witness id to equal the committed transaction id.
9. O2A writes the witnesses. The seal witness is `[signature, leaf script, control block]`. The fee witness is a key-path signature. Both inputs get an empty `final_script_sig`.
10. `extract_tx` must keep the same txid. If the txid moves, the commitment is wrong and the spend stops.

The binary supplies the seal input itself. It does not call the rgb-api wallet payment builder and it does not construct `RgbWallet`.

## Workspace isolation

rgb-protocol 0.11.1 and RGB-WG 0.12 do not share a Cargo graph.

| Workspace | What it contains |
| --- | --- |
| Repository root | `o2a-demo-core`, plus the archived 0.12 crates so that graph still resolves |
| `default-members` | `crates/o2a-demo-core` only |
| `exclude` | `spikes/rgb-0.11.1` |
| `spikes/rgb-0.11.1` | its own workspace, members `["check"]`, crates.io pins at exactly `0.11.1` |

`o2a-demo-core` has no RGB dependency. Build the adapter with its own manifest. Do not add it to the parent members list. Do not enable rgb-api features `bp`, `bdk`, or `fs`. Do not use rgb-lib for seal custody. If both lines are present on one machine, cross the `secp256k1` boundary with validated bytes.

## C6 and C7, enforced by O2A

RGB 0.11.1 does not drop a consignment because a later plain spend, or a spend of another contract on the same outpoint, is missing from that consignment.

C6 mined a plain spend of the seal, with no RGB commitment. Re-validation of the earlier consignment still printed `Consignment is valid`. The declarative right stayed listed on the spent outpoint, and the witness was `None`.

C7 issued two contracts on one outpoint with the same O2A digest. Both validated while the outpoint was unspent. After only the second contract was spent, the first consignment still validated.

O2A reports that fact in its own layer. `evaluate_lineage` in `crates/o2a-demo-core/src/chain.rs` returns `SEAL_CLOSED_WITHOUT_VALID_TRANSITION` when all of these are true:

- the scripts match the seal policy;
- the seal-creating transaction and the spend are on the named chain at the required depth;
- the current seal is spent;
- `o2a_ok` is false, which means no valid O2A transition closes that seal.

The RGB line stays a separate field. It can still say the consignment is valid. The regtest lineage records the same split: a plain close is `SEAL_CLOSED_WITHOUT_VALID_TRANSITION` while its consignment remains valid, and a contract left behind on a shared seal is `SEAL_CLOSED` while the continued contract stays `CURRENT`.

Each RGB check calls `consignment.validate` twice and keeps the text only when the two results match. Each O2A check calls the evaluator twice the same way. That in-process pair is not two laptops and not two Bitcoin backends.

## How to run it

From the repository root:

```text
cargo build --manifest-path spikes/rgb-0.11.1/Cargo.toml --locked --bin rgb011-check
```

The binary is `spikes/rgb-0.11.1/target/debug/rgb011-check`. Its commands are `lineage`, `plan`, `genesis --seal TXID:VOUT`, `claim`, and `verify`. With no command it runs the compatibility check. `signet-genesis` and `signet-claim` remain as aliases. Each alias sets no network and prints a deprecation note. `signet-verify` is the same kind of alias for `verify`.

The default recovery delay inside `o2a-demo-core` is 10 blocks, and the default entity index is 0. A ceremony sets `O2A_DEMO_DELAY`, `O2A_DEMO_THRESHOLD`, and `O2A_DEMO_ENTITY` before `plan` and again before `genesis`. A different delay is a different address. Rehearsal 3 used delay `1008`, threshold `2`, and entity `31`. Delay `1007` produced the unfunded address `tb1pxlm4rc3nl8pd8ervyc273tyjzrhflczf0kn59rht5ncq4un53t6qj9l23t`.

`plan` prints the seal address and the exact policy. The operator's wallet pays that address. `genesis --seal TXID:VOUT` reads the funding transaction through the profile's backend. It signs only when the output exists, the script matches the policy recomputed for that outpoint, every input sequence is `4294967294` or `4294967295`, the output is unspent, a header and merkle inclusion proof are present, and the confirmation count has reached the profile depth. That depth is 1 on regtest, 1 on signet, and 6 on mainnet. Otherwise it prints `funding sequences … are replaceable; genesis stays unsigned` or `depth N; genesis stays unsigned until D confirmations`. `verify` uses that same profile depth. The recorded regtest lineage evaluator keeps its own depth of 2.

`createrawtransaction` treats replaceable as true unless the fourth argument is false. `fundrawtransaction` with `"replaceable": false` does not rewrite a sequence that is already on the input. Set the sequence on the input. For an output that must keep its txid, use `4294967294`, locktime `0`, and `false` as that fourth argument. Rehearsal 3 funded with `sendtoaddress` and `replaceable=false`, which produced sequence `4294967294`.

Regtest lineage requires `O2A_DEMO_SEED_FILE`, `O2A_DEMO_NETWORK=regtest`, `RGB_CHAIN=regtest`, and `RGB011_EVIDENCE`. Start it only on a chain whose height is already at least 101. The recorded port chain has H0 `108`, hash `3f7e4a7404599036d852d03aa68136da66ad0e36726c9f8c61a826385b64966c`. Leave every block at or below that height in place.

The stage commands, the cookie path, and the two-laptop stage rule are in [block0-runbook.md](block0-runbook.md). The seed file and the RPC cookie stay outside the repository. On mainnet, `plan`, `genesis`, and `claim` need `--authorize-mainnet` and the typed word `mainnet` before any key is derived. `verify` does not. A mainnet `plan` requires `O2A_DEMO_SEED_FILE`. On regtest or signet, `O2A_DEMO_UNSAFE_PREVIEW=1` is the only request that prints an address from the published unsafe seed, and that command prints its banner. The binary allows plan, verify, genesis, and the one `official_name` claim. It refuses transitions. Configured mainnet endpoints are read-only, and every mainnet broadcast is refused. The operator's wallet funds the seal.

## Evidence

| Bundle | What it holds |
| --- | --- |
| `evidence/regtest-rgb011-compat-2026-09-28` | C1–C9. The original NO-GO text stays in that file. |
| `evidence/regtest-rgb011-compat-2026-09-28-addendum` | D14 license rerun. Licenses are report-only. |
| `evidence/regtest-rgb011-lineage-2026-09-28` | Regtest lineage, including the C6/C7-style O2A reports. |
| `evidence/signet-block0-rehearsal-3-2026-09-28` | Signet rehearsal on this adapter. |
