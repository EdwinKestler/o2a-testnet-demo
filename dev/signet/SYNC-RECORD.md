# Signet sync record

First time bitcoind and electrs stayed on the same fully synced public-signet tip.

| Field | Value |
| --- | --- |
| Date and time (UTC) | 2026-09-24T15:55:57Z |
| bitcoind version | Bitcoin Core v31.1.0 |
| electrs version | v0.12.0 |
| bitcoind image | `bitcoin/bitcoin:31.1` |
| bitcoind digest | `sha256:da25cedc66b1daefff9f412ee196c901a899c3fa68a33b20849c3e08b5c40d63` |
| electrs image | `getumbrel/electrs:v0.12.0` |
| electrs digest | `sha256:83cb57d7b23f553b3bb858f47e784018a723993777508f2299dc07c2fccb9db0` |
| Chain | signet |
| Block height | 323515 |
| Best block hash | `0000000cdafc6aa896a7aa788aa93b76697256a1af9fce945d77b0d294e15818` |
| electrs indexed height | 323515 |
| Elapsed sync time | 1h 44m 4s, from bitcoind start `2026-09-24T14:11:53Z` to this match |
| bitcoind volume | 28G, 29391150476 bytes (`du /data`) |
| electrs volume | 1.4G, 1436471842 bytes (`du /data`) |
| Host OS | Ubuntu 24.04.5 LTS, Linux 7.0.0-34-generic x86_64 |
| Docker version | 29.8.1 |

Genesis `00000008819873e925422c1ff0f99f7cc9bbb232af63a077a480a3633bee1ef6` is the default public signet. No wallet was created.

bitcoind finished initial block download at about 15:38 UTC. The clean electrs index started at 15:51:10Z, reached that tip at 15:54:45Z, and kept running after RocksDB compaction. Later blocks were new signet blocks, not a second download. At the following check the shared tip was height 323518, hash `0000000a653c2403ae63afa116b6347f52dd246b5214bd876317b830de2b9b0b`.
