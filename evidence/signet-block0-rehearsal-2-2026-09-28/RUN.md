# Signet block-0 rehearsal 2

Disposable signet identity. It closes no Phase 0 gate and spends no mainnet coins.

**Date:** 2026-09-28

**Authority:** `o2a-protocol` `b622c9830e98085c5270a604dc14fa7bec1bf2c2`

**Verifier:** commit `f6a8406be77146aeb3f4b9a7fc71f5d8c69167bc`. The seal script is recomputed from the seal policy in the signed genesis. The seal record only locates the outpoint. No seal record was edited by hand.

## Why the first run was superseded

`evidence/signet-block0-rehearsal-2026-09-28` is unchanged. Gate 3 rejected it because the verifier trusted `scriptPubKey` from the seal record. This run uses a new seed, entity index 2, and a new seal.

## Result

| Step | Expected | Observed | Result |
| --- | --- | --- | --- |
| Node, electrs, and Blockstream | Same height | 324083 at the start | PASS |
| Toolchain route to signet electrs | TCP open | `toolchain_electrs=open` | PASS |
| Wrong delay 10 | Different address | `tb1pe78sl32x48um3v8v87wpfds33z46n2mkqhkrn89gkwwlhzvsv9nq97gzag` | PASS |
| Core address check | Matches the plan | `tb1p05yh6dyxjecertln2p48aj3zn8073qgaw0al9qzccegljhp96vksf87xce` | PASS |
| Funding | Non-replaceable, within 20,000 sats | 10,000 sat output, both input sequences `4294967294` | PASS |
| Depth | 6 confirmations | 6, anchor height 324084, verifier height 324089 | PASS |
| Offline genesis | Zero `signer_entity` | EntityID `95f84ba2dd5e8140cfd50d881e41ed07250e715c0a6f8d388247de8a58efcf51` | PASS |
| Two verifiers, no seed mounted | CURRENT, consignment imported | Both CURRENT, reports match | PASS |
| `official_name` | Same EntityID and state id | Claim signature valid. State id `f02951489439e0855a0fbd5232f96f9c5b4add5106477daacf37fbb18fb7709e` | PASS |
| Restore from the backup alone | CURRENT, no seed | CURRENT, claim valid | PASS |
| Venue network down | No CURRENT | Connection refused to `127.0.0.1:9` | PASS |
| Seal left unspent | No rotation or close | No later spend | PASS |

The first identity `ea7248ccdbbbdd4cde1690a2a489d8106a60567f903e514d75e02cc3b30e8039` was not reused.

## Stage clock

| Step | Seconds |
| --- | ---: |
| Plan | 0.523 |
| Wrong-policy check | 0.531 |
| Fund | 0.204 |
| Wait for 6 confirmations | 4552.879 |
| Sign genesis | 0.538 |
| Anchor the consignment | 1.216 |
| Two verifies | 1.678 |
| Projector | 0.070 |
| Sign the name claim | 0.470 |
| Projector claim line | 0.022 |
| Restore verify and claim | 1.301 |

On-stage work after the wait stayed under five minutes.

## Regtest re-check with this verifier

Live re-verify of the 2026-09-28 regtest packages, without rewinding the chain:

| Package | Filed final state | Re-verify |
| --- | --- | --- |
| Entity 1 seal D | CURRENT | CURRENT |
| Entity 1, no observation | INCOMPLETE | INCOMPLETE |
| Fork genesis on the spent seal | SEAL_CLOSED_WITHOUT_VALID_TRANSITION | SEAL_CLOSED_WITHOUT_VALID_TRANSITION |
| Entity 2 after the plain close | SEAL_CLOSED_WITHOUT_VALID_TRANSITION | SEAL_CLOSED_WITHOUT_VALID_TRANSITION |
| Entity 3, no genesis file | INVALID | INCOMPLETE |

Entity 3 changed because a package with no signed genesis is now INCOMPLETE. The old INVALID result came from recomputing a script with demo keys. The depth-6 PENDING snapshot cannot be reproduced on the later chain tip.

## Wallet

Balance after this seal payment: 0.00114699 signet bitcoin. This rehearsal spent 12,101 sats, including the 10,000 sat seal.
