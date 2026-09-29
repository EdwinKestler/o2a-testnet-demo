# Signet block-0 rehearsal 3

Disposable signet identity. It closes no Phase 0 gate and spends no mainnet coins.

**Date:** 2026-09-28

**Authority:** `o2a-protocol` `0a8d54f30b431661adefdbf1d4cdb10a42eca47a`

**Carrier:** rgb-protocol 0.11.1, close method Opret, `ChainNet::BitcoinSignet`

The earlier bundles `evidence/signet-block0-rehearsal-2026-09-28` and `evidence/signet-block0-rehearsal-2-2026-09-28` were left unchanged. This run uses a new seed, entity index 31, and a new seal. Rehearsal 2's EntityID `95f84ba2dd5e8140cfd50d881e41ed07250e715c0a6f8d388247de8a58efcf51` was not reused.

## Result

| Step | Expected | Observed | Result |
| --- | --- | --- | --- |
| Node, electrs, and Blockstream | Same height | 324129 at signing | PASS |
| Fresh entity index | Public plan, seed outside the repo | Index 31, delay 1008, threshold 2 | PASS |
| Wrong delay 1007 | A different, unfunded address | `tb1pxlm4rc3nl8pd8ervyc273tyjzrhflczf0kn59rht5ncq4un53t6qj9l23t` | PASS |
| Core address check | Matches the plan | `tb1pcvdv0m82jkj70xxlut5a7yqzr4p7lmqtm3x2py338h892r2th02scy38ay` | PASS |
| Funding | Non-replaceable | 10,000 sats, sequence `4294967294` | PASS |
| Depth | 6 confirmations | 6, anchor block 324124, tip 324129 | PASS |
| Genesis | `signer_entity` is 32 zero bytes | EntityID `bc7c1031ad9daf1a30d2edd1b4dc91079c89ccb8bd8997e6c08fb97570861e37` | PASS |
| Two verifiers, seed unset | CURRENT | Both CURRENT at height 324129. The reports match | PASS |
| `official_name` | This EntityID, claim valid | `Rehearsal Name`. State id `45e11a2bd42c0dbca3f06bcf5bec672e92d08dc2615c6172699923ad9feb3e9a` | PASS |
| Restore from the public package | CURRENT, claim valid, seed unset | CURRENT, `name_claim=valid` | PASS |
| Seal left unspent | No rotation or close | `gettxout` still returns the output | PASS |

## Identity

| Field | Value |
| --- | --- |
| Entity index | 31 |
| Network byte | 3 |
| Threshold | 2 of 3 |
| Delay | 1008 blocks |
| EntityID | `bc7c1031ad9daf1a30d2edd1b4dc91079c89ccb8bd8997e6c08fb97570861e37` |
| State id | `45e11a2bd42c0dbca3f06bcf5bec672e92d08dc2615c6172699923ad9feb3e9a` |
| Seal | `513f69c7d07b1bf6b664f2ca6e723938a73fd8ea8052d8b1ccc07a2ecdbfcede:0` |
| Address | `tb1pcvdv0m82jkj70xxlut5a7yqzr4p7lmqtm3x2py338h892r2th02scy38ay` |
| Display name | Rehearsal Name |

Public keys are in `raw/plan.txt`. The root x-only is `1176eb9c0c29d0287e3f745e02f79cf84bb7090b2b1d12401cb2006d2c64d824`.

## Funding

The show wallet is `rehearsal`. The payment used `replaceable=false`. No fee bump was sent.

| Field | Value |
| --- | --- |
| Amount | 10,000 sats |
| Sequence | `4294967294` |
| Confirmations at signing | 6 |
| Anchor block | 324124 |
| Anchor hash | `00000011ebb467178dfac1055a719a0013e14bf01b9697c86b2d3ad74001addd` |

The wait from the first zero-confirmation reading at 21:32:31 to 6 confirmations at 22:11:06 was 38 minutes 35 seconds. After that, signing through the restore finished in under 2 seconds. `raw/timings.txt` is the clock from the start of that signing script.

Trusted balance of wallet `rehearsal` after the payment: 0.00103301 signet bitcoin.

## How the checks ran

Verifier A and verifier B were two clean directories on this machine. Each ran `signet-verify` with the seed variable unset. Both talked to the same signet node and the same electrs. They are not two physical laptops.

The restore was a third clean directory. It held the signed genesis, the consignment, the public plan, and the name claim. The seed variable was unset. The state was CURRENT and the claim verified.

Each RGB check called `consignment.validate` twice and kept the text only when the two results matched. Each O2A check did the same with the lineage evaluator. Headers come from one electrs instance. That is a trust assumption, not a light client.

The projector page is local: `/tmp/block0-screen/index.html`. A test page with a labeled test id was built and removed before the real EntityID was shown. The real page shows the identity line, the name, the EntityID, the QR code of that EntityID, both CURRENT lines, and "Name claim signed: Rehearsal Name". A copy is `raw/projector.html`. It was not copied into `site/`.

## Files

| Path | Contents |
| --- | --- |
| `public/genesis.o2a` | Signed genesis |
| `public/genesis.strict` | RGB consignment |
| `public/claim.o2a` | `official_name` claim |
| `public/public.txt` | EntityID, state id, seal, confirmations |
| `raw/plan.txt` | Public plan |
| `raw/verify-a.txt`, `raw/verify-b.txt` | The two CURRENT reports |
| `raw/restore.txt` | Restore report |
| `MANIFEST.sha256` | Hashes of the files in this directory |

## Secret scan

A scan of this directory for extended private-key headers and for the seed-phrase keyword was empty. The seed file and the RPC cookie are absent.
