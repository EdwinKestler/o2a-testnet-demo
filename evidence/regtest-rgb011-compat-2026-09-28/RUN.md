# RGB 0.11.1 regtest compatibility check

Regtest only. Date of the run: 2026-09-28 local, 2026-09-29 UTC in the node logs. This bundle records one scored chain. It does not adopt a dependency and it does not change `o2a-protocol`.

## Verdict

NO-GO

C1 through C7 passed. C8 failed because `hex_lit 0.1.1` is MITNFA. C9 is a report. The decision memo is `docs/rgb-0.11.1-compat-memo.md`.

## Isolation

- Worktree: `../wt-rgb011-spike`. Branch: `spike/rgb-0.11.1` from `71471ee`.
- The original checkout stayed on `feat/block0-rehearsal`. `o2a-protocol` main was not edited.
- Cargo workspace: `spikes/rgb-0.11.1/`, with its own `Cargo.lock`. The parent workspace excludes that directory. The parent `Cargo.lock` is unchanged.
- `o2a-demo-core` is a path dependency. `o2a-demo-seal` and `o2a-demo-rgb` are not dependencies.
- Compose project: `o2a-rgb011`. Subnet: `172.30.34.0/24`. Host ports: `127.0.0.1:18445` to bitcoind `18443`, and `127.0.0.1:50021` to electrs `50001`.
- Signet and the other demo stacks were left running. This run did not spend the signet rehearsal wallet.

## Published crates

All nine files below were present in the local crates.io cache. The sha256 of each `.crate` file matches `Cargo.lock`. The list is also in `raw/crate-checksums.txt`.

| Crate | Version | sha256 |
| --- | --- | --- |
| rgb-consensus | 0.11.1 | `cecb9e63175ae5f75b53724c8fbdea261d5903086a121fe722fbd8af3b3b1af3` |
| rgb-ops | 0.11.1 | `60f23b8116d672ea7e455de1cb340cea243ff40b319c4b3c3622efe88f46ecff` |
| rgb-schemas | 0.11.1 | `3d46b2d581f785bb8f69760df64d2d0bbdcc78839817560ff4e73aed5f9e1cb6` |
| rgb-api | 0.11.1 | `aad6e1bf00a974bd19e3e9102fe0c185ac8eda9ef550378c1a3585245c2e0006` |
| rgb-psbt-utils | 0.11.1 | `b1c22ce8b1f6e125c5125a16f136c93450ceb0830e9048a43dad336c1b588120` |
| rgb-aluvm | 0.11.1 | `4b371aad57bb172879ef661d1cc102dadca47ff703def8d916b31ca772180960` |
| rgb-strict-encoding | 1.0.4 | `784229a9cf3ed198e59cbd44bc5a3c772619e5f2a864eea3ec13524ec4eb1616` |
| rgb-strict-types | 1.0.4 | `7cb1f717f2235eedfb147cfd5a47ccdb6660168e38f1acf2afcb199d19104e2f` |
| bitcoin | 0.32.102 | `bb0ce8bd5baaa0d303a19915a6d93afed161f528654e42da2a7a97d05c59499a` |

`rgb-api` default features are empty. This binary does not enable `bp`, `bdk`, or `fs`. It does not depend on `rgb-lib` and it does not construct `RgbWallet`.

`Cargo.lock` has 139 packages. Two are path packages (`o2a-demo-core`, `rgb011-check`). The lock has no git source, no release-candidate version, and no alpha version.

## Chain

H0 is 101. That is the height after the 101-block maturity mine and before the first seal-funding transaction.

Electrs 0.12.0 answers `blockchain.headers.subscribe` only after its index has a tip. The run mined 101 blocks with `bitcoin-cli` before starting electrs, then waited until the log said `indexed 102 blocks`, and only then connected.

Bitcoin Core rejects one transaction that pays the same address twice. The four seal scripts share one address, and the three fee inputs share another. Funding used one transaction per output, then mined those transactions together. Funding confirmation F is 108. Every funding vout is 0.

| Output | Amount, sats | Txid |
| --- | --- | --- |
| Seal A | 100000 | `bc4f9ef97f730bab952329ff694f47ce0060f2e8b299711013ffddc339b79fd1` |
| Seal C | 120000 | `472bdcb06477f19be995ef64813494c6a941ac97ff09dab15ca73a30d521dabf` |
| Seal D | 130000 | `58e2f6454e6874c50f242d2b7169402b94bff6d9c930654c5941fa35d23a30b5` |
| Seal R | 110000 | `ebc849dc9a9f55c3071b1bb4527cb747353d21d5fa0cd86c13ca20e3cf12fb85` |
| Fee for C4 | 60000 | `038408b09bba57c200a2d3f897a996aecf3e9dc8db15d7a8b6feba272bd0b5be` |
| Fee for C5 | 61000 | `46c14adbc29d375ffbd1a1693f86722468ea69305dc7fd96fec783069dc200c4` |
| Fee for C7 | 62000 | `a885e7bb470f4815c49a0e53ff4845e29006f43f258f5c5feda941596c73a4de` |

Core 31 `getrawtransaction` verbose output and electrs `blockchain.transaction.get` verbose output include `confirmations` and `blockhash`. They omit the block height. The resolver calls `getblockheader` on that block hash and reads `height` and `time`. Two independent `ElectrumResolver` opens produced byte-identical validation text. See `c4-validators.txt`.

## Identity used for C1

Fresh non-demo seed, entity index 9, through `o2a-demo-core`. The seed file stayed outside the repository.

| Field | Value |
| --- | --- |
| Schema | `rgb:sch:oqE1HKzG_NrzhV2M0tn~mkfJfic5ztF6iUr8s8YbdDE#ivan-robin-exotic` |
| NIA schema, for comparison | `rgb:sch:RWhwUfTMpuP2Zfx1~j4nswCANGeJrYOqDcKelaMV4zU#remote-digital-pegasus` |
| Entity | `2820fdbb0a22c370c5e7c3b47b48be2cbb9e32dba0f5eb7be636dee704bed0ff` |
| Digest | `cffe41dd6305f0461b5363faeae33ea0cb8c7dfef5d0680a74e24f7b15e94c57` |
| State | `28a62c653206506675ee047549249a6e435677f3042157539162c0ad507d22d5` |
| Contract | `rgb:VWn__XsZ-pVGALkp-~x09Pp7-tsPls0w-AbOe8WU-V8odbX0` |
| Seal | `bc4f9ef97f730bab952329ff694f47ce0060f2e8b299711013ffddc339b79fd1:0` |
| Root xonly | `3144d4d7d965fc55af45c4d0852458dae125dc5d601a702bb55a6640ffeea2c6` |
| Offline revoke opid | `5d8d1c7b7aa2951bc801927327737ad162716cfefcd8bb48b8c65bb3306dd601` |

The offline revoke was built and not broadcast. Validators are `None`. The O2A script matched the seal. The RGB seal type carries an outpoint and has no script field.

## Case lines

The full expected, observed, and result lines are in `cases.txt`. Heights below are from that file.

| Case | Result | Chain fact |
| --- | --- | --- |
| C1 | PASS | Genesis report: `Consignment is valid` |
| C2 | PASS | `o2a_script_match true` |
| C3 | PASS | Opret commit on witness `7827144701eb3c772e04fd9bc019aebc2ad16bc4fce89a73385ad8870ab3baa6` |
| C4 | PASS | Mined at 109. `validators_agree true` |
| C5 | PASS | Rejected at 109 as `non-BIP68-final`. Allowed at tip 117. Mined at 118. |
| C6 | PASS | Mined at 119. Report: `Consignment is valid`. Witness `None`. |
| C7 | PASS | Both contracts valid while unspent. After spending contract 2, mined at 120, contract 1 still lists the genesis right. |
| C8 | FAIL | See the dependency gate below. |
| C9 | REPORT | Script-tree host: `use of taproot script descriptors is not yet supported.` Key-only host built a Tapret proof and was not broadcast. |

C5 uses the same Bitcoin rule as the 2026-09-25 smoke in `evidence/regtest-seal-tapscript-smoke-2026-09-25/bip68.txt`: `non-BIP68-final` before maturity, then acceptance. Seal R was confirmed at height 108. The relative lock is 10. Inclusion is in the block at height 118.

C6 matches that smoke's S6 record: re-validation prints no error, and the previous consignment stays valid. The 0.11.1 text is `Consignment is valid`, with the right still on the spent outpoint and witness `None`.

## Wallet-ownership lines hit by C3

The binary supplied the external seal input itself. It called the PSBT embed and commit methods. It did not call the wallet payment builder.

- `rgb-api-0.11.1/src/pay.rs:112-122`. `ContractOutpointsFilter::should_include` returns false unless `wallet.filter_unspent().should_include` is true and `stock.contract_assignments_for` is non-empty.
- `rgb-api-0.11.1/src/pay.rs:535-537`. `CompositionError::InsufficientState` when that selection is empty.
- `rgb-api-0.11.1/src/pay.rs:554-556`. `set_rgb_close_method`, `set_as_unmodifiable`, `rgb_embed`.
- `rgb-api-0.11.1/src/pay.rs:570`. `transfer` calls `rgb_commit`.
- `rgb-api-0.11.1/src/wallet.rs:49-94`. `RgbWallet` wraps a `WalletProvider`. The `bp` load path starts at line 69.

The rust-bitcoin `Psbt` methods used here do not check wallet ownership. A stock query for the fee outpoint was empty. A stock query for the seal outpoint found the assignment.

## Dependency gate (C8)

`cargo audit --file Cargo.lock` loaded 1273 advisories, scanned 139 crates, and exited 0. `paste` is absent, so the `RUSTSEC-2024-0436` ignore was not used. Log: `raw/audit-and-first-deny.txt`.

`cargo deny` 0.20.2 was run three times:

1. The first file set `vulnerability = "deny"`. cargo-deny 0.16 removed that key. The process exited 1 before the license scan. Same log as the audit.
2. After deleting that key, and before `Zlib` was on the allow list, deny exited 4. Advisories, bans, and sources passed. Licenses failed on `foldhash 0.1.5` (`Zlib`) and `hex_lit 0.1.1` (`MITNFA`). Log: `raw/deny-zlib-and-mitnfa.txt`.
3. `Zlib` is on the 2026-09-24 allow list in `deny.toml` at the repository root and in `o2a-protocol/docs/22-license-and-adoption-assessment.md`. It was added to the spike allow list. Deny was run again. Advisories ok, bans ok, sources ok, licenses FAILED, exit 4. The only rejection is `hex_lit 0.1.1`, license `MITNFA`, pulled by `bitcoin 0.32.102`, which `rgb-consensus 0.11.1` depends on. `hex_lit` is a normal dependency in `bitcoin-0.32.102/Cargo.toml`, not an optional feature. Log: `raw/deny-mitnfa.txt`.

`MITNFA` is a stop in `DEMO-GATE.md` rule 6 and in the 2026-09-24 license decision. It was not added to the allow list. Duplicate-version warnings, including `secp256k1` 0.29.1 and 0.33.1, are warnings because `multiple-versions = "warn"`.

The parent 0.12 `Cargo.lock` has no `hex_lit` package. The 2026-09-25 smoke's `cargo deny check` exited 0.

## Files

| File | Contents |
| --- | --- |
| `cases.txt` | Scored case lines, ending in `checks_finished` |
| `A.strict`, `C.strict`, `D1.strict`, `D2.strict`, `R.strict` | Genesis consignments |
| `c4-rotate.hex`, `c4-transfer.strict`, `c4-validators.txt` | Controller spend |
| `c5-recover.hex`, `c5-transfer.strict`, `c5-validators.txt` | Recovery spend |
| `c6-plain.hex` | Plain spend |
| `c7-contract2.hex`, `c7-transfer.strict`, `c7-validators.txt` | Contract 2 spend |
| `raw/rgb011-check.txt` | The same stdout as `cases.txt` |
| `raw/audit-and-first-deny.txt` | Audit exit 0 and the removed-key deny error |
| `raw/deny-zlib-and-mitnfa.txt` | License failures before `Zlib` was allowed |
| `raw/deny-mitnfa.txt` | Scored deny: MITNFA only |
| `raw/crate-checksums.txt` | Published crate checksums |
| `MANIFEST.sha256` | Hashes of the files in this directory |

## Secret scan

A scan of this directory found no extended private keys, no RPC cookie, and no seed file. Public xonly keys, txids, and consignments are included. The artist seed and the RPC cookie stayed in `/tmp`.

The chain was left at height 120. Do not invalidate blocks at or below H0.
