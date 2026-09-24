# Signet acceptance — 2026-09-24

Public signet only. bitcoind ran with `-disablewallet=1`. No wallet, key, or address was created. The stack was not changed again while this bundle was written.

Images: `bitcoin/bitcoin:31.1` digest `sha256:da25cedc66b1daefff9f412ee196c901a899c3fa68a33b20849c3e08b5c40d63`, client v31.1.0. `getumbrel/electrs:v0.12.0` digest `sha256:83cb57d7b23f553b3bb858f47e784018a723993777508f2299dc07c2fccb9db0`, electrs v0.12.0. Host: Ubuntu 24.04.5 LTS, Linux 7.0.0-34-generic x86_64. Docker 29.8.1.

Genesis `00000008819873e925422c1ff0f99f7cc9bbb232af63a077a480a3633bee1ef6` is the default public signet.

## Three electrs failures before the clean index

1. At the first start, before the replacement at 2026-09-24T14:24:36Z, electrs exited 1 with `Error: An unknown argument '--auth' was specified.` This build accepts the password only as the config-file key `auth`. The first container still passed `--auth` on the command line.
2. At 2026-09-24T15:38:19.240Z electrs logged `Cannot assign requested address` (os error 99) on `GET /rest/block/<hash>.bin` and exited at 2026-09-24T15:38:19.248Z. bitcoind REST sends `Connection: close`, and electrs opens a new connection per block, so the container ran out of local ports. The container was recreated at 2026-09-24T15:42:16Z with `net.ipv4.tcp_tw_reuse=1` and local ports `1024 65535`.
3. That recreated index reached the tip and then failed compaction. At 2026-09-24T15:45:46.632Z RocksDB logged `Compaction error: Corruption: Compaction sees out-of-order keys` on the `script_hash` column. The process exited at 2026-09-24T15:47:35.274Z. The index had already been interrupted by failure 2. Only the electrs volume was removed. A clean index started at 2026-09-24T15:51:10.568Z, reached height 323515 at 2026-09-24T15:54:45.424Z, logged `started auto compactions` at 2026-09-24T15:54:45.557Z, and stayed up. The new RocksDB log had no out-of-order error.

## Steps

1. `docker compose config --quiet` exited 0 at the 2026-09-24T16:12Z collection. bitcoind started at 2026-09-24T14:11:53.676Z with restart count 0 and stayed that process until the intentional step-7 stop. The config-file electrs process started at 2026-09-24T14:24:36.139Z and still had restart count 0 when the five-minute mark, 2026-09-24T14:29:36Z, was recorded at 2026-09-24T14:34:36Z. The clean electrs process started at 2026-09-24T15:51:10.568Z and still had restart count 0 when it was stopped for step 7 at 2026-09-24T16:13:47Z.
2. The first status match that stayed up was 2026-09-24T15:55:57Z: chain `signet`, height 323515, electrs height 323515, hash `0000000cdafc6aa896a7aa788aa93b76697256a1af9fce945d77b0d294e15818`. Elapsed time from the bitcoind start was 1h 44m 4s. At 2026-09-24T16:12Z the data volumes were 29391150476 bytes (bitcoind) and 1436471842 bytes (electrs). The 20-line tails from that check are [bitcoind-tail.log](bitcoind-tail.log) and [electrs-tail.log](electrs-tail.log). The tip had moved to 323518 by then because new signet blocks arrived.
3. `python3 scripts/status.py` exited 0. Output is [status.txt](status.txt), captured at the 16:12Z check before step 7.
4. A raw TCP connection to `127.0.0.1:60601` sent `{"id":0,"method":"server.version","params":["test","1.4"]}` and a newline. The reply is [server-version.txt](server-version.txt).
5. `dev/signet/SYNC-RECORD.md` was filled with the 15:55:57Z match.
6. `printf 'no\n' | sh scripts/reset.sh` printed `Nothing was deleted.` and exited 0. Volume creation times stayed 2026-09-24T08:10:35-06:00 and 2026-09-24T09:51:10-06:00. The byte sizes stayed 29391150476 and 1436471842. Both containers kept their start times.
7. `docker compose down` removed the containers and the network and left both volumes. `docker compose up -d` started them again. bitcoind started at 2026-09-24T16:13:47.044Z, loaded block-index heights 322825...323518, and logged `Leaving InitialBlockDownload` at 2026-09-24T16:13:47Z. electrs started at 2026-09-24T16:13:52.681Z and at 2026-09-24T16:13:53.133Z logged `height=323518 headers loaded` for hash `0000000a653c2403ae63afa116b6347f52dd246b5214bd876317b830de2b9b0b`. `status.py` exited 0 on that same tip. electrs did not index from height 0.

Volume names, creation times, and byte sizes are in [volumes.txt](volumes.txt).
