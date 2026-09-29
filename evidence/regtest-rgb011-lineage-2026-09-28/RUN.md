# RGB 0.11.1 regtest lineage

Disposable demo-lineage evidence; no Phase 0 gate closure.

Run date: 2026-09-28 local. Network: isolated Bitcoin regtest. Carrier: rgb-protocol 0.11.1, close method Opret. Depth required by this run: 2. Recovery delay: 10 blocks.

This bundle is the maintained-adapter lineage. The artist seed and the RPC cookie stayed in `/tmp`.

## Isolation

| Item | Value |
| --- | --- |
| Worktree | `../wt-rgb011-port` |
| Branch | `feat/rgb-0.11.1-adapter` |
| Compose project | `o2a-rgb011-port` |
| Subnet | `172.30.35.0/24` |
| Host bitcoind | `127.0.0.1:18446` |
| Host electrs | `127.0.0.1:50022` |
| Volumes created | `o2a-rgb011-port_bitcoin-data`, `o2a-rgb011-port_electrs-data` |
| Network created | `o2a-rgb011-port_regtest` |
| Images | `o2a-rgb011-bitcoin:latest`, `o2a-rgb011-electrs:latest` (reused) |
| Authority read for this run | `0a8d54f30b431661adefdbf1d4cdb10a42eca47a` |

`o2a-rgb011`, `signet-infra`, `o2a-testnet-demo`, and `o2a-phase0` stayed up. This run spent no signet coins and created no mainnet transaction.

A funding attempt earlier on this same chain stopped before any genesis signature. This bundle is the later run. Its H0 is 108. The earlier funding transaction is unused.

## Chain floor

| Field | Value |
| --- | --- |
| H0 | 108 |
| H0 hash | `3f7e4a7404599036d852d03aa68136da66ad0e36726c9f8c61a826385b64966c` |
| Funding confirmation F | 110 |
| Tip after the run | 129 |
| Tip hash | `23e9248cd012e4175d03bc989a2cff4644b6c5cedebdd206d24a9ed357b1cd1c` |

The reorg case invalidated that tip. Height fell to 128. `reconsiderblock` restored height 129. The invalidated block is above H0.

Headers come from one electrs instance. That is a trust assumption, not a light client.

## Validators

Every RGB check called `consignment.validate` twice, through two Electrum resolver opens. The two texts must match. Every O2A check called the evaluator twice. The two texts must match. A mismatch fails the case. Each `*-validators.txt` file is the agreed RGB text.

Bitcoin Core `getdescriptorinfo` and `deriveaddresses` matched the local seal address for entities 11, 12, 13, 15, and 16. A mismatch stops the run before that seal is funded. Entity 14 signs the same resulting state as entity 13, so it uses entity 13's seal address.

## Result

| Case | Expected | Observed | Result |
| --- | --- | --- | --- |
| RBF of a non-seal payment | The txid changes | `3ac3b01acc5b2bf47ec45e945362ef8fe8ef99411d9597e9960a726799e8256a` then `dfd628c76b3d1207c7ec65f094a47ef04812c479757a52ae4126cfd67148d34f` | PASS |
| Seal funding sequences | `4294967294` or `4294967295` | Every seal and fee input is `4294967294` | PASS |
| CPFP of the entity 11 change | Parent txid stays the same | Parent `28d630c0a2a2e35b97931b6f4d0a7ad5248026ded5a59539a2e993db2e681af4`, child `3b650a23dbb9fbd0413cf082b33cb7372dd602112248eea3e24d9168bd2f2aad`, change vout 1 | PASS |
| Entity 11 genesis at F | PENDING_CONFIRMATION | PENDING_CONFIRMATION at height 110 | PASS |
| Same genesis one block later | CURRENT | CURRENT at height 111 | PASS |
| Entity 11 rotation | Successor CURRENT | Spend `5db435c0ba9dcd051d4aec4d6e1d76440650dcc972169b2d119bf6b10d9a4299` mined at 112. CURRENT at 113 | PASS |
| Entity 12 recovery, BIP68 delay 10 | Rejected before maturity, then CURRENT | Early reject `non-BIP68-final`. Allowed at 119. Mined at 120. CURRENT at 121 | PASS |
| Entities 13 and 14, one seal | Distinct EntityIDs. The one left behind is SEAL_CLOSED. Its consignment still validates | Ids differ. Continued CURRENT. Other SEAL_CLOSED. `rgb_other valid` | PASS |
| Entity 15, two contracts, one seal | The contract left behind is SEAL_CLOSED. RGB still validates it | Same O2A digest. Continued CURRENT. Other SEAL_CLOSED. Both consignments valid | PASS |
| Entity 16 plain close | SEAL_CLOSED. RGB genesis still valid | `SEAL_CLOSED_WITHOUT_VALID_TRANSITION` at height 129. Consignment valid | PASS |
| Reorg above H0 | Height stays at least 108, then the tip returns | 129 to 128 to 129. `above_h0 true` | PASS |

Full expected, observed, and result lines are in `cases.txt`. The same stdout is `raw/lineage.txt`.

## Identities

Each identity uses its own entity index. The seed file is outside the repository.

| Label | Index | EntityID | Genesis state | Seal |
| --- | --- | --- | --- | --- |
| e11, then rotation | 11 | `5acacad52c3e80d15970e70f914a5a6fe0c9f2e82936862e216c636e2d652818` | `7c9b72c094282b359b4ac15ca0043a6b0ad6351cd6f03d9bb51541de173aa6fe` | `28d630c0a2a2e35b97931b6f4d0a7ad5248026ded5a59539a2e993db2e681af4:0` |
| e12 recovery | 12 | `be3551e477661f7c23d4ef0be6d9b67726fe33d8ec92c0e2f6a0ff3219ff50e5` | `348dee36cec5786e9b51dff79dd46797809075184f29ab546d07443c983adab3` | `76bf1e5226133f8086b1128d2fce055a54a7d1dfb132d4e6c2aa3fb7f61e5775:0` |
| e13 continued | 13 | `57c1b47a62821707030aafc9727ae898a6689e5c690caa25678b85bcd1c72885` | same resulting state as e14 | `f0124378bc8c2ac90a268c15d2d297fadf7fb94f692d0242a824a155f7849092:0` |
| e14 left behind | 14 | `81defbdac6da2ca34035cd3f7fd9957828af5766731bec33529476273543896b` | same resulting state as e13 | same outpoint as e13 |
| e15 reissue | 15 | `0199abf14bed9205cc216bb8562bdeb4e7fbe6716df2c8c2c07edb452068c5bf` | `e24ae49ef2af804511eb1ffea2fbc3114f24f47055c86cff611df1ae6c22910c` | `054a541ea5b49c625052ccb71cc2271df78024d5b5c59e67ca2f4b3797528cf9:0` |
| e16 plain close | 16 | `be6a9b85565f08f842785db6987ebf006518d6ecf7b0a6a78e969ed661b8ccd6` | `fceb355451eb2e7e8ed5271a4bfc136057549619ff4b23bd88370ac2d46f70e1` | `74a16d053b23747e75a1e15d526e4864db2c7857f67fd47d7af11ab483604c77:0` |

| Contract | Id | Role |
| --- | --- | --- |
| e13 | `rgb:5Kot2y65-QkNQKFx-qYaKrbU-FjONSpt-YQ~LtGd-5C_KXyU` | Continued by the rotation |
| e14 | `rgb:AdxOAJOH-UGko29y-iIMQxro-FKO1QQl-ikdVZql-ZbRlypI` | Left on the spent seal. O2A reports SEAL_CLOSED. RGB still validates the genesis consignment |
| e15 continued | `rgb:j8zWGUjf-j3AL0Ct-ssvo~6h-3nU9Gvt-Td9JneS-yIKUits` | Second contract, same O2A digest, issued at timestamp 1759017605 |
| e15 left behind | `rgb:hBguMDYZ-SAr39BT-8XXhf82-zuAYW4B-NTcJLhE-98IHLw4` | First contract, same digest, issued at timestamp 1759017604 |

Recovery spend: `a55b9560f075b2a1ad7563c7ea3b2447bf6dc218bf6537ba190fb5ada64621f9`, mined at height 120. The seal was confirmed at height 110. Delay is 10, so the spend is mined at `funded_at + delay`.

Same-seal rotation spend: `8d7c0f25de75cd7375b2645cb5035925afeceaff9a19b9ac31057e477670dacf`, mined at 122.

Plain close spend: `67271fd9ec8c58a792884a6e8faf9df274f1d2343303f3b80d8756decbff6058`, mined at 128. The O2A report is at height 129.

## Funding

Seal outputs are 200000 sats. Fee inputs are 80000 sats. The plain-close seal is 150000 sats. Every listed input sequence is `4294967294`.

| Output | Txid |
| --- | --- |
| e11 seal | `28d630c0a2a2e35b97931b6f4d0a7ad5248026ded5a59539a2e993db2e681af4` |
| e12 seal | `76bf1e5226133f8086b1128d2fce055a54a7d1dfb132d4e6c2aa3fb7f61e5775` |
| e13 seal | `f0124378bc8c2ac90a268c15d2d297fadf7fb94f692d0242a824a155f7849092` |
| e15 seal | `054a541ea5b49c625052ccb71cc2271df78024d5b5c59e67ca2f4b3797528cf9` |
| e16 seal | `74a16d053b23747e75a1e15d526e4864db2c7857f67fd47d7af11ab483604c77` |
| e11 fee | `1d4651a398d6221bfd58655f7bff10b57e6afa09c41df55fe13260a13f2e7883` |
| e12 fee | `3ba08f620c17f0c21d7bdca7b6afaa766dda44934e5a120586579d91f4ad5f61` |
| e13 fee | `c85b7285d755224965d376b2d9ba0def030673523cd168bbea91c183aea42418` |
| e15 fee | `15ec19230e71cd5bb28d0e5295fdb73cffd52aa1e86f6543a2622185bc9ea6df` |

Entity 11 funding has two inputs, both final. The CPFP child spends change vout 1 of that funding transaction. The parent txid stayed the same. No seal funding transaction was fee-bumped.

## Addresses

| Entity | Address |
| --- | --- |
| 11 | `bcrt1p3k05zv7vc6yqr52s3urw2tmwyu86jrphn7uyp5nu4778w0cv2sqqpm7jcy` |
| 12 | `bcrt1px823gslddwgzugcjfmdaqzwwayr5956mgqmjjv99yz4ltggxsejqhngsxq` |
| 13 and 14 | `bcrt1p6zvxr7x9hspve02gk2rum7yfxmd7xae93cu42s2xfpuewgqxstqsxny2gg` |
| 15 | `bcrt1pv6v78gvjvuq7vc6l5xf0s6ud0g7ha7wcps7vw3phyz566d6rgh5shyzg4v` |
| 16 | `bcrt1puvygl86vsf846h8x48ndmlw2dsfdg72k46qvn79nsyzk8lmv4lmqze0pdh` |

Descriptors are in `cases.txt`. NUMS x-only is `50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0`. The fee key x-only is `63d1db1088ec2fd9d83d429e1e098414c506c3ddce588db614b533b97796d16a`.

## Files

| File | Contents |
| --- | --- |
| `cases.txt` | Scored lines, ending in `lineage_finished` |
| `raw/lineage.txt` | The same stdout, plus the process exit line |
| `e11-*`, `e12-*`, `e13-*`, `e14-*`, `e15-*`, `e16-*` | O2A objects, consignments, spend hex, and validator text |
| `fund-*.hex` | Funding transactions |
| `MANIFEST.sha256` | Hashes of the files in this directory |

## Secret scan

A scan of this directory for extended private-key headers and for the seed-phrase keyword was empty. The seed file and the RPC cookie are absent. Public x-only keys, txids, and consignments are included.

The chain was left at height 129. Leave every block at height 108 and below in place.
