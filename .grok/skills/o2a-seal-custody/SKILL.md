---
name: o2a-seal-custody
description: Build, sign, finalize and verify O2A seal spends (custom P2TR tapscript seals) correctly, including the verifier trust rule.
when-to-use: When touching seal construction, PSBTs, seal spending, or verification code.
paths: crates/**, spikes/rgb-0.11.1/**
---

# Seal custody

## Script (from the spec; never improvise)
P2TR with the BIP341 NUMS internal key and no key path. One `<xonly> OP_CHECKSIG` leaf per capability-2 controller seal key. A recovery leaf `and_v(v:multi_a(k,R...),older(d))` (Core compiles it as `... OP_k OP_NUMEQUALVERIFY <d> OP_CHECKSEQUENCEVERIFY`). Leaves are reduced pairwise left to right, carrying an odd final node.

## Maintained 0.11.1 adapter
The maintained adapter lives in its own Cargo workspace, `spikes/rgb-0.11.1`. The parent workspace excludes that directory. Do not put rgb-protocol 0.11.1 and the archived 0.12 crates in one dependency graph.

Use the rgb-api PSBT layer. Do not construct `RgbWallet`. Do not enable the rgb-api features `bp`, `bdk`, or `fs`. Do not use rgb-lib for seal custody.

`commit_opret` builds a version-2 transaction: an empty `OP_RETURN` (`0x6a`) first, then the next seal output, then change. The seal input keeps the sequence passed in. The fee input uses `Sequence::MAX`. The PSBT carries `witness_utxo` on both inputs, the NUMS `tap_internal_key`, and `tap_merkle_root`. It then calls `set_rgb_close_method(CloseMethod::OpretFirst)`, `set_opret_host`, `set_as_unmodifiable`, `push_rgb_transition`, and `rgb_commit`. The fascia witness id must equal the committed txid.

O2A finalizes after that commitment. The seal witness is `[sig, leaf script, control block]`. The fee witness is the key-path signature. Both inputs get an empty `final_script_sig`. `extract_tx` must leave the txid equal to the fascia witness id.

Final-funding-sequence guard: `sequences_final` accepts only `4294967294` and `4294967295`. `signet-genesis` refuses to sign when any funding input is smaller and prints `funding sequences … are replaceable; genesis stays unsigned`. `createrawtransaction` treats its replaceable argument as true unless the caller passes false. `fundrawtransaction` with `replaceable: false` does not rewrite a sequence already set on an input. Set the input sequence explicitly (`4294967294` when the output must stay, `4294967293` only for a replaceable non-seal payment) and pass locktime `0` plus the replaceable bool as the fourth `createrawtransaction` argument.

## Archived 0.12 adapter
- O2A owns seal tracking. The RGB wallet only funds fees and never sees or selects a seal.
- The PSBT seal input carries `witness_utxo`, `tap_internal_key` (NUMS), `tap_merkle_root`, and `tap_leaf_script`.
- Finalize script-path inputs manually: the witness is `[sig, leaf script, control block]`. For recovery, `[sig or empty per key in reverse order, script, control block]`.
- Set an explicit empty `final_script_sig` before `extract()` (bp-std v0.12.0-rc.3 panics without it; its finalizer also takes the first merkle sibling as the leaf hash). See `docs/upstream-needs.md` in o2a-protocol.
- Recovery spends: `nVersion` 2, `nSequence` = delay. BIP68 counts from the seal output's confirmation.

## Verification (hard rule; lesson from commit abf769d)
- The verifier MUST derive the expected scriptPubKey from the seal policy of the SIGNED state that names the seal: `script_for_named_seal` / `seal_for_state` on a decoded signed object.
- NEVER take the script from a seal record or any package-supplied field. NEVER locate policy bytes by substring search. A seal record only locates the outpoint.
- No genesis means the history is never CURRENT. CURRENT also requires a sourced unspent observation of the current seal (source, block hash, height).
- Demo keys (`demo_genesis_state`, `demo_rotation_state`, `state_for_stage`) may appear only in fixture and spend-builder code. They must never be used to verify; replace them with signed-state decoding before any real transition.

## Fees
Never RBF a seal funding transaction yourself: a new txid changes the outpoint and therefore the EntityID. Use CPFP. Since Bitcoin Core 28, nodes accept replacements regardless of signaling, so the real protection is waiting for depth before the genesis is signed. The 0.11.1 guard above is that wait, enforced in `signet-genesis`.
