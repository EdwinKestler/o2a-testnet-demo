# Regtest seal-policy lineage

This is disposable demo-lineage evidence. It closes no Phase 0 gate.
The 2026-09-24 bundles and signet-acceptance are the superseded pre-seal-policy lineage.

**Run date:** 2026-09-26

**Network:** isolated Bitcoin regtest through one local electrs instance

**Spec:** sibling `../o2a-protocol` at `c7b08716d017d1f6125e6a728fb098673a09d433`

Headers come from one electrs instance. That is a trust assumption, not a light client.
Required depth on regtest is 1.
An anchor absent from the named best chain at that depth is not current.
A reorg that removes the anchor drops dependent transitions.

The recorded chain floor is height 104 (`raw/H0`). No block at or below 104 was invalidated.
The only invalidated block is height 125, hash `3a9c074e8eea02d4601f20a7d0c2af8ff993b9d8ca3b29983b37f877d760a39d`.
`reconsiderblock` put that block back.

## What was run

| Fixture | Identity | Result |
| --- | --- | --- |
| F1 genesis on seal A | 1 | CURRENT |
| F2 controller rotation A to B, plus a stale binding | 1 | CURRENT, and demo-core rejects the stale binding |
| F3 recovery of B before the delay, then rotation B to C | 1 | `non-BIP68-final`, then `bad-txns-inputs-missingorspent`, then CURRENT |
| F4 recovery C to D after C's height plus 10 | 1 | CURRENT |
| F5 reorg of the rotation anchor | 2 | CURRENT, then INCOMPLETE, then CURRENT |
| F6 plain close of seal B | 2 | SEAL_CLOSED_WITHOUT_VALID_TRANSITION |
| F7 genesis names an outpoint whose script is not the policy | 3 | INVALID |
| F8 verify with no current-seal observation | 1 | INCOMPLETE |

Each verify ran in two fresh validator directories. The report lines are byte-identical.
The files are `raw/*-identical.txt`.

Bitcoin, RGB, and O2A are separate lines in each report.
RGB import does not decide the identity state. The state comes from the explicit proofs in demo-core.

## Core address check

Bitcoin Core `getdescriptorinfo` and `deriveaddresses` returned the same address the CLI printed.

| Stage | Address | Checksum |
| --- | --- | --- |
| Genesis, controller 0 | `bcrt1pkxp9f8n0eh0ld2xtnqh4flm433f8ckv4g44yjsef6c9rtsmlfapsm5w6ls` | `cllmtlup` |
| Rotation and recovery, controller 1 | `bcrt1pmwhl43kqn54v6m78s93xfy55z3qfzpzd74xy5jjga4avg2xgpxkqgh59wg` | `x82k5ny2` |

Seals B, C, and D are separate outpoints on the rotation address.
Identity 2 used the same two addresses. The Core output is in `raw/f1-core-crosscheck.txt`, `raw/f2-core-crosscheck.txt`, `raw/f5-genesis-core-crosscheck.txt`, and `raw/f5-rotate-prepare-core-crosscheck.txt`.

## Identity 1

Contract `contract:mPgNX4HK-bAnSf6V-iOyTKZE-JcV8Eyc-KyA8ZsV-fBUL5ws`.

| Seal | Outpoint | Height |
| --- | --- | --- |
| A | `49dfd7c8626a6cd524e26f2dc1d5357b8725ae62fc61611f5b3139bbdd7dfa3e:0` | 105 |
| B | `4ead506a5bac0baed381b28530cb326679b9122649de40d6f4d2711d1936601c:1` | 106 |
| C | `6bd16fe445887f96c84ae6c579053f5ee079d1b907502ade76cf2cf9200fc0a6:1` | 109 |
| D | `217c29edd7837960c439df5cebc5272a2f3ca6f013f473caca726bc9c7840476:1` | 112 |

| Step | Anchor txid | Confirmed at |
| --- | --- | --- |
| Genesis funding of A | `49dfd7c8626a6cd524e26f2dc1d5357b8725ae62fc61611f5b3139bbdd7dfa3e` | 105 |
| Rotate A to B | `772eb81d83913dc230db3ea97bb71c28670392768acefe561fbc18e331f881c1` | 108 |
| Rotate B to C | `21b225bb7f4dea1860917975559cbb9038ce3de6707aaf5a851d35c3107ae3ce` | 111 |
| Recover C to D | `b51c83e6ab36d426eb6054e9b0a436365db043d8f668cf637106f150b6a7e1a7` | 121 |

F1 at height 105 reported CURRENT. The best block hash was `1ff368c0d3c809858d721f4495fdfff41231802b93f5136c23e4ab23e3be3970`, the same hash Bitcoin Core returned. The two filed reports are in `raw/f1-verify-a.txt` and `raw/f1-verify-b.txt`. They were written before `lineage.txt` named the consignment, so the RGB line says `consignment absent`. The next run imported `id1/genesis.rgb` and kept the same Bitcoin and O2A lines. That stdout is `raw/f1-verify-imported.txt`.

The wallet did not contain seal A (`raw/f1-wallet.txt`).

F2 built an offline transition that still bound controller 0 after the controller set had moved to controller 1.
Demo-core rejected it: `controller seal bindings do not cover the transition key set`.
The digest is `9f6f5744e9615bd290fc76145ee6d2282675307dca99c4a5e7cdaf5f2ac90aed` (`raw/f2-stale.txt`).

F3 signed a recovery-leaf spend of B with `nSequence` 10.
`sendrawtransaction` returned `non-BIP68-final`.
Controller 1 then spent B. The same raw recovery returned `bad-txns-inputs-missingorspent`.

C confirmed at height 109. The recovery of C was broadcast after the chain reached height 119, which is C's height plus 10. The recovery confirmation is height 121. The verifier reported CURRENT for seal D.

F8 verified D with no current-seal observation. The result is INCOMPLETE. The Bitcoin line is `current seal was not observed`.

## Identity 2

Contract `contract:nKjqQ5l_-3qOUD_J-vlzHvQ0-0eCNyZq-Lf0xdrX-9G4K3m8`.

| Seal | Outpoint | Height |
| --- | --- | --- |
| A | `f651fcf0af09b6542078e5266083a1e94d6cd365276e2b7872607eedd0230e44:0` | 122 |
| B | `481ec137b39a68d4843e36198cfd5bf50ab90074a5e3a91267da4ef37dacd8c3:0` | 123 |

The rotation anchor is `d262bd240ae84604cd41229e4a1154a8da5b5279edad4fe8760e8a1a5f245501` in block `3a9c074e8eea02d4601f20a7d0c2af8ff993b9d8ca3b29983b37f877d760a39d` at height 125.

Before `invalidateblock`, seal B was CURRENT at tip 125.
After `invalidateblock`, the tip was 124 and the anchor was not on that best chain. The result was INCOMPLETE.
After `reconsiderblock`, the tip was 125 again and seal B was CURRENT.

F6 spent B with a script-path transaction that carries no RGB commitment.
The txid is `2d3dd8b397183deb31c04dc51c7f176030676b23d17cfdca603ea4ba8e16e77f`.
After one confirmation the verifier reported SEAL_CLOSED_WITHOUT_VALID_TRANSITION.
The O2A line is `no valid transition closes the seal`.

## Identity 3

The outpoint `43a3d54bb60b78aaa0d5e49d0a3ed330a81d9835fb50f5aa38c08f139ce31b99:1` pays a witness program that is not the genesis policy script.
The verifier reported INVALID. The Bitcoin line is `script does not match the policy`.
The offline genesis digest for that outpoint is `73ce76cf7fb8e65b7a0cf36bc40202b2a2e1188abb20a496bace1892559be821`.

## Files

`id1/` and `id2/` hold the consignments, the signed O2A objects, the raw anchor transactions, the seal records, and `lineage.txt`.
`id3/seals/A.txt` is the mismatched seal record.
`raw/` holds the command output.
`work/` is local chain state and is not part of this bundle.
