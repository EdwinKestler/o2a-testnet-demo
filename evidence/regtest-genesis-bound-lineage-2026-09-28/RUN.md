# Regtest genesis-bound lineage

Disposable demo evidence. It closes no Phase 0 gate.
The 2026-09-26 seal-policy bundle was not edited.

**Run date:** 2026-09-28

**Network:** isolated Bitcoin regtest, one local electrs

**Spec:** sibling `../o2a-protocol` at `0ef16c2132ea54cdfd4aa86a34a748f998f388d8`

Headers come from one electrs instance. That is a trust assumption, not a light client.
EntityID is `TaggedHash("O2A/v0.1/entity-id", genesis payload)`.
The genesis `signer_entity` is 32 zero bytes.
Each identity uses its own entity index under `m/1'/index'/...`.

The recorded chain floor is height 105 (`raw/H0`).
Block 104 is `41ed9acd4ff0a6ffd68b049a07da00cefeb6e03e3c606b368b1468c4aa86057e`.
No block at or below 105 was invalidated.
The only invalidated block is height 126, hash `6406bc9a3212919d3b683676ab05fb2c66fd65708e1bcc6288600c21c8568694`.
`reconsiderblock` put that block back. The chain tip at the end of the run was 129.

Electrs had exited on a 404 for block `3ae8e48d4ae7f7ffed7b00dd303bc947a1fdd9a067177ba9c4a7ff65a405b2b8`, which is not on this chain.
It was started again on a fresh index of the chain Core has.
The first F7 verify ran before that index had the new block and returned `missing result`.
That stdout is `raw/e3-mismatch-electrs-lag.txt`.
The filed F7 report is the retry, after electrs height matched bitcoind.

## Result

Each verify used two fresh validator directories.
`raw/*-identical.txt` records `byte_identical=true`.

| Case | Expected | Observed | Result |
| --- | --- | --- | --- |
| Entity 1 genesis, depth 6, best height 106 | PENDING_CONFIRMATION | PENDING_CONFIRMATION, consignment imported | PASS |
| Same genesis at best height 111, depth 6 | CURRENT, same EntityID | CURRENT, `3ec753a7b05168f107415522437e52163bea1f596527752d849fc5f33de1fddb` | PASS |
| Rotation to seal B | EntityID unchanged, CURRENT | `history_entity` matches, CURRENT at height 114 | PASS |
| Recovery to seal D | EntityID unchanged, CURRENT | `history_entity` matches, CURRENT at height 125 | PASS |
| Second genesis, same seal and same root, after the legit rotation | Different EntityID, then SEAL_CLOSED_WITHOUT_VALID_TRANSITION | Fork id `73b301b9b8c7e0c0b29baa4972200353b088d7ccee2adf18e108465a45f89c0d`, SEAL_CLOSED, consignment imported | PASS |
| That fork claims the legit EntityID | It does not | Verifier reports the fork id | PASS |
| Entity 2 reorg of the seal-creating block | INCOMPLETE, then CURRENT | INCOMPLETE at height 125 while the block was invalidated; CURRENT at height 126 after reconsider | PASS |
| F6 plain close of entity 2 | SEAL_CLOSED_WITHOUT_VALID_TRANSITION | SEAL_CLOSED at height 127, consignment imported | PASS |
| F7 script is not the policy | INVALID | INVALID, `rgb=consignment absent` | PASS |
| F8 verify with no current-seal observation | INCOMPLETE | INCOMPLETE at height 125, consignment imported | PASS |
| RBF of the seal funding transaction | EntityID changes; the old transaction is not confirmed | `f73fbbd24fcdf16f1b71d2000116fa83120c0de60ee174497766226c487193a2` then `6dfb53ad9f4a593f55a41944cc991339c81679cc7b4288c5a64217e4778e71f6`; old txid absent from mempool and chain | PASS |
| CPFP spends the change of a non-RBF funding transaction | EntityID unchanged; parent confirms | `83f4dae5d18da115a0d364a307b9f5983d58426719de1b847085b339938c75fa` before and after; parent confirmations 1 | PASS |

F7 has no consignment because no contract was issued. The output is a normal address, so the Bitcoin line is the script mismatch. The 2026-09-26 F7 report has the same RGB line.

## Identities

Core `getdescriptorinfo` and `deriveaddresses` matched the CLI address for entities 1, 2, and 4.

| Entity | Index | Address | EntityID |
| --- | --- | --- | --- |
| 1 genesis | 1 | `bcrt1px8q3f498nqkzpx2hypwg2mjc7fqy4htk4yms4xsap833cmncn95qwld58r` | `3ec753a7b05168f107415522437e52163bea1f596527752d849fc5f33de1fddb` |
| 1 rotation | 1 | `bcrt1p6qlp7waaeumwruh2x4t6sulnv5u4pd69vwxaz7mwxtfsj8zn9vzs9kqmpt` | same id |
| 2 | 2 | `bcrt1ptp7qqp7ha7fsxa3x8nmzacxmc0mlm9m4t8wppx2dgj276hrlzeusjk7ryn` | `aca129959b83c0b6780bd265ec5a18fde8cb04f81e8810624ecb7f8ca348cd89` |
| 3 | 3 | plain address `bcrt1qqeqtvgzj9uhkfvalj8cld5unw6y79ua6d5c582` | no genesis object |
| 4 RBF, then CPFP | 4 | `bcrt1padlwgtxh0z83mwh236q94gfe582xhpjrs46pgcnvgwtkr80xhd6ssrt5ve` | changes on RBF; CPFP id above |

Entity 1 contract: `contract:uc6ay4tR-thAzlii-_kGWNf8-FSdTf3L-TSl~Vkm-cQ~DG6w`.
Genesis seal: `d39822450db62d9ecb3f90fa578679b5d983ee98dbf0b2749aa3d37072620902:0`.
Paths start at `m/1'/1'/...`.

Entity 2 contract: `contract:5zRkbEeQ-rliabXU-QjSWceR-CYYAVvI-1zRfZ1s-2VY_6c8`.
Genesis seal: `8840e2fc281303150f41f411e1bd6e3edebe356485b5dfcf164d4bb7abe04b52:0`.
Paths start at `m/1'/2'/...`.

The RBF funding input sequence is `4294967293`.
The CPFP parent input sequence is `4294967294`.
The replaced txid is `fbeb8ad0b712f2457d8e5e2fbe7078ad33e0eff1424a9ca3630726eecf3eeb76`.
The replacement txid is `bdc90a03892e918431e2778b0712fd8edb83196527d3cf74e9dc3dcb38c0e856`.
The CPFP parent txid is `f44e1ba21c75777fee68a0d5508eb517ab9ed2ca2958e1c9b642c3b064502f2b`.

## Signet, same day, separate chain

Measured after the regtest run, before this bundle was committed.
Local signet `getblockcount` was 324049.
`https://blockstream.info/signet/api/blocks/tip/height` was 324049.
Wallet `rehearsal` balance was `0.00000000`.
The receive address set aside for faucet coins is `tb1qusen4tc36dhqtc0cy48zkdaz40gkzhpmc7sfpe`.
Public faucet pages that pay the default signet asked for a browser challenge. No challenge was bypassed, so no signet coins were received.
`signet.bitsaga.be` returned txid `e45cf0e7b8a2bced025c79d0960aceebd2a6d81883748f806bab6f10b13970cc`. Blockstream signet returns 404 for that txid.
