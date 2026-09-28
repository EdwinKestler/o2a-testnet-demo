---
name: o2a-seal-custody
description: Build, sign, finalize and verify O2A seal spends (custom P2TR tapscript seals) correctly, including the verifier trust rule.
when-to-use: When touching seal construction, PSBTs, seal spending, or verification code.
paths: crates/**
---

# Seal custody

## Script (from the spec; never improvise)
P2TR with the BIP341 NUMS internal key and no key path. One `<xonly> OP_CHECKSIG` leaf per capability-2 controller seal key. A recovery leaf `and_v(v:multi_a(k,R...),older(d))` (Core compiles it as `... OP_k OP_NUMEQUALVERIFY <d> OP_CHECKSEQUENCEVERIFY`). Leaves are reduced pairwise left to right, carrying an odd final node.

## Spending
- O2A owns seal tracking. The RGB wallet only funds fees and never sees or selects a seal.
- PSBT seal input: witness_utxo, tap_internal_key (NUMS), tap_merkle_root, tap_leaf_script.
- Finalize script-path inputs manually: the witness is [sig, leaf script, control block]. For recovery, [sig or empty per key in reverse order, script, control block].
- Set an explicit empty final_script_sig before extract() (bp-std v0.12.0-rc.3 panics without it; its finalizer also takes the first merkle sibling as the leaf hash). See docs/upstream-needs.md in o2a-protocol.
- Recovery spends: nVersion 2, nSequence = delay. BIP68 counts from the seal output's confirmation.

## Verification (hard rule; lesson from commit abf769d)
- The verifier MUST derive the expected scriptPubKey from the seal policy of the SIGNED state that names the seal: script_for_named_seal / seal_for_state on a decoded signed object.
- NEVER take the script from a seal record or any package-supplied field. NEVER locate policy bytes by substring search. A seal record only locates the outpoint.
- No genesis means the history is never CURRENT. CURRENT also requires a sourced unspent observation of the current seal (source, block hash, height).
- Demo keys (demo_genesis_state, demo_rotation_state, state_for_stage) may appear only in fixture and spend-builder code. They must never be used to verify; replace them with signed-state decoding before any real transition.

## Fees
Never RBF a seal funding transaction yourself: a new txid changes the outpoint and therefore the EntityID. Use CPFP. Since Bitcoin Core 28, nodes accept replacements regardless of signaling, so the real protection is waiting for depth before the genesis is signed.
