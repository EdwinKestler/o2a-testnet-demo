Disposable demo-lineage evidence. This is not a Phase 0 gate closure. No normative seal script is adopted.

This note corrects the 2026-09-25 smoke bundle. That bundle is unchanged.

# Abandoned first genesis

The first `issue` command printed:

```text
contract_id=contract:vxQyGKPR-KIYaYQo-ibssSXr-KiNZxva-8tVy9Sv-0sjW_IU
genesis_cell=xd1ao6Ji8_YSoQilgruvK0kgyc04YLTH6ITG78IJOsY:0
genesis_seal=46475f1eb02b18b99e9975dbfd833c6f403c3649ed4ef8b55f28541359e30878:0/0bb50495dbccfaf436a88f2bc99d1a981497cc52df45fe04981cec6431abe9caffffffffffffffff
o2a_digest=246b8b2c0f5377ddebdb285deb19da8f83a999b9119fd888f9e9d30519573164
```

The only seal outpoint that output names is funding transaction `46475f1eb02b18b99e9975dbfd833c6f403c3649ed4ef8b55f28541359e30878` output 0. The same outpoint is seal A in the later bundle. `state.txt` in that data directory, read before the directory was cleared, was:

```text
contract_id=contract:vxQyGKPR-KIYaYQo-ibssSXr-KiNZxva-8tVy9Sv-0sjW_IU
cell=xd1ao6Ji8_YSoQilgruvK0kgyc04YLTH6ITG78IJOsY:0
seal=46475f1eb02b18b99e9975dbfd833c6f403c3649ed4ef8b55f28541359e30878:0
digest=246b8b2c0f5377ddebdb285deb19da8f83a999b9119fd888f9e9d30519573164
```

Data directory: `evidence/regtest-seal-tapscript-smoke-2026-09-25/work/issuer`. The contract files were under `O2AIdentity.vxQyGKPR-KIYaYQo-ibssSXr-KiNZxva-8tVy9Sv-0sjW_IU.contract` in that directory. A read-only search of the surviving issuer directory finds no `vxQyGKPR` bytes. The directory now holds only `contract:zf3wG3sS-CX225e8-ppsigBg-rOP4UrJ-XBGNp8e-Ab1HX5c`.

The next anchor command on that contract panicked before it printed a raw transaction:

```text
thread 'main' (1) panicked at /home/o2a/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/psbt-0.12.0-rc.3/src/data.rs:793:55:
non-finalized input
```

That log contains no `raw_tx`, no txid, and no `sendrawtransaction`. Nothing from that attempt was broadcast.

The last retained runtime view, taken after the panic and before the contract directory was removed, was:

```text
runtime_update=ok
owned name=identity cell=xd1ao6Ji8_YSoQilgruvK0kgyc04YLTH6ITG78IJOsY:0 seal=46475f1eb02b18b99e9975dbfd833c6f403c3649ed4ef8b55f28541359e30878:0/0bb50495dbccfaf436a88f2bc99d1a981497cc52df45fe04981cec6431abe9caffffffffffffffff status=Genesis
```

The contract directory was then removed. A listing immediately after that removal shows only the `wallet` directory. The second genesis, `contract:zf3wG3sS-CX225e8-ppsigBg-rOP4UrJ-XBGNp8e-Ab1HX5c`, was issued into the same directory on the same seal outpoint. The successful A→B anchor `db952ddece17f82905e8f562f1b62b3d877189403780efd7a8d18d6b16f08fc3` spends that outpoint and is confirmed (12 confirmations at the read-only query; `gettxout` on output 0 is empty). The first contract's files were already gone before that spend. The surviving stockpile has no state for `vxQyGKPR`, so the effect of the spend on that contract's RGB state is not recoverable from retained data.

# Subnet override

`dev/compose.yaml` is restored to `main`. Its regtest subnet is `172.30.30.0/24`. The smoke subnet `172.30.32.0/24` is only in `dev/compose.smoke.yaml`, using Compose `!override` on `networks.regtest.ipam.config`.

A rerun uses both files:

```text
docker compose -p o2a-seal-smoke --file dev/compose.yaml --file dev/compose.smoke.yaml --profile rgb up --detach --wait bitcoin electrs
```

`dev/bitcoin.conf` on this branch still sets `rpcallowip=172.30.32.0/24`. That line was not part of this restore. The existing `o2a-seal-smoke` network and volumes were not recreated or removed.
