# Signet block-0 rehearsal

Disposable signet identity. It closes no Phase 0 gate and spends no mainnet coins.

**Date:** 2026-09-28

**Authority:** `o2a-protocol` `b622c9830e98085c5270a604dc14fa7bec1bf2c2`

**Stand-in:** one host played the artist device, the operator, two verifiers, and the restore directory as separate folders. The fresh seed stayed in `/tmp/o2a-artist-device` and is not in this bundle. A second recovery share was written outside the repository. Staff received no share.

## Result

| Step | Expected | Observed | Result |
| --- | --- | --- | --- |
| Node, electrs, and Blockstream | Same height | 324065 at the start | PASS |
| Toolchain route to signet electrs | TCP open | `toolchain_electrs=open` | PASS |
| Recovery offer | 2 of 3, delay 1008, staff hold none | threshold 2, delay 1008, staff file says none | PASS |
| Wrong delay at pre-flight | Different address | delay 10 address differs from the funded address | PASS |
| Core `deriveaddresses` | Matches the plan | `tb1pkvtggwcy2zua0gyjw3789matlvhsaxe2mm066q7632hdlmj2d5mqu54euy` | PASS |
| Funding | Non-replaceable, about 20,000 sats | 10,000 sat output, sequence `4294967294`, fee 1,490 sats | PASS |
| Depth | 6 confirmations | 6, anchor height 324067, verifier height 324072 | PASS |
| Offline genesis | Zero `signer_entity`, EntityID | `ea7248ccdbbbdd4cde1690a2a489d8106a60567f903e514d75e02cc3b30e8039` | PASS |
| First verify | CURRENT | INVALID, script did not match the policy. The seal record lacked the artist keys | FAIL, then corrected |
| Verify after the record was rewritten | Two validators CURRENT, consignment imported | Both CURRENT at 324072, reports match | PASS |
| `official_name` claim | Predicate `official_name`, same EntityID and state id | Claim signature valid. State id `0adb657d225cbdf398ce530981068489ea6f5c2514c37691f8ec7d10fc369105` | PASS |
| Projector | EntityID, two CURRENT lines, name claim | `raw/projector.html` | PASS |
| Restore from the backup alone | Same EntityID and CURRENT | CURRENT, claim valid, no seed mounted | PASS |
| Venue network down | Verify cannot reach the node | `electrum 127.0.0.1:9: Connection refused` | PASS |
| Seal left unspent | No rotation or close | No later spend | PASS |

Entity index 1. Contract `contract:hmmVcxzV-tZWyrcH-ar~EiC3-z9V6iIm-VVn6uby-S8A0KOo`.
Seal `51c541bccced93f7722d0171997e08669c96cfdf448f301e840ff03082571d14:0`.

## Stage clock

The depth wait is not part of the five-minute stage.

| Step | Seconds |
| --- | ---: |
| Plan | 0.502 |
| Wrong-policy check | 0.491 |
| Fund | 0.186 |
| Wait for 6 confirmations | 3700.423 |
| Sign genesis | 0.538 |
| Anchor the consignment | 1.252 |
| Two verifies | 1.771 |
| Projector | 0.065 |
| Sign the name claim | 0.501 |
| Projector claim line | 0.018 |
| Restore verify and claim | 1.415 |

On-stage signing and checks, after the wait, took well under five minutes.

## Checker

`check_genesis_freeze.py` was run on the host against the demo's 63 freeze outputs. It printed `63/63 outputs unchanged`, including the Rust EntityID, state id, and BIP340 cross-checks. The same checker is not run inside the demo image: that image's `/tmp` cannot execute the crypto-checker's Cargo build script.

## Wallet

Rehearsal wallet balance after the seal payment: 0.00126800 signet bitcoin. The payment spent 11,490 sats, including the 10,000 sat seal and a 1,490 sat fee.
