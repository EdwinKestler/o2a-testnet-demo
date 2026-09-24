# Signet infrastructure

Public Bitcoin signet only. This stack runs a full node and an Electrum server.
It does not create a wallet, a key, or an address. `bitcoind` is started with
`-disablewallet=1`. Getting coins from a signet faucet is a manual step.

## Images

| Service | Image | Digest |
| --- | --- | --- |
| bitcoind | `bitcoin/bitcoin:31.1` | `sha256:da25cedc66b1daefff9f412ee196c901a899c3fa68a33b20849c3e08b5c40d63` |
| electrs | `getumbrel/electrs:v0.12.0` | `sha256:83cb57d7b23f553b3bb858f47e784018a723993777508f2299dc07c2fccb9db0` |

`getumbrel/electrs:v0.12.0` is romanz electrs 0.12.0. Its `--network` help lists
`signet`. `blockstream/electrs` is not published on Docker Hub.

Host ports are `127.0.0.1:38332` for signet RPC and `127.0.0.1:60601` for
Electrum, so they do not use the regtest ports. Signet P2P `38333` is open
only on the Compose network. This electrs build has no P2P flag. It reads blocks from bitcoind's REST
interface, so bitcoind also runs with `-rest=1`. It authenticates with the
`auth` config setting, using the same `rpcauth` user and password. That
setting is not accepted on the electrs command line.

Expect about 10–25 GB for the bitcoind volume with `txindex` and about 2–8 GB
for the electrs index. A first sync often takes 30–180 minutes and must be
stopped if it is still unfinished after 6 hours. Exact sizes and time go in
`SYNC-RECORD.md`.

## Choices

- Bitcoin Core image is `bitcoin/bitcoin:31.1`, which is newer than 28.x and was present locally. The running client reports v31.1.0.
- Electrs image is `getumbrel/electrs:v0.12.0` (romanz electrs 0.12.0). Its `--network` list includes `signet`. `blockstream/electrs` is not on Docker Hub.
- This electrs build does not connect to bitcoind's P2P port. It uses JSON-RPC and the REST interface. Bitcoind still listens on signet P2P `38333` inside the Compose network so the node itself can sync. `-rest=1` is set because electrs fetches blocks over REST.
- The electrs `auth` value is written to a config file. This build rejects `--auth` on the command line.
- The Compose subnet is `172.30.31.0/24` so it does not overlap the repository regtest network `172.30.29.0/24`. RPC is published only on `127.0.0.1`. `-rpcallowip` allows `127.0.0.1` and that subnet. A host connection arrives from the Compose gateway, which is inside the subnet.
- The bitcoind healthcheck calls `bitcoin-cli` with the datadir cookie. That cookie is created by bitcoind. No wallet is created.
- The RPC user name is `signetrpc`.
- Containers have no restart policy, so a crash stays stopped.
- electrs runs as the image user `electrs` (uid 1000).
- bitcoind REST answers with `Connection: close`. electrs 0.12 opens a new connection per block, so the container sets `net.ipv4.tcp_tw_reuse=1` and a wider local port range. Without that, indexing stops with `Cannot assign requested address`.
- electrs log filter is `info` so index progress is visible.

## Start

```bash
cd signet-infra
export DOCKER_CONTEXT=default
python3 scripts/gen_rpcauth.py
docker compose config --quiet
docker compose up -d
```

`gen_rpcauth.py` writes `.env`. That file is gitignored. `.env.example` has
placeholders only.

## Check

```bash
python3 scripts/status.py
```

Exit 0 means bitcoind reports chain `signet`, initial block download is
finished, headers equal blocks, and the electrs tip equals that height.

## Stop

```bash
docker compose down
```

Data volumes are kept.

## Reset

```bash
sh scripts/reset.sh
```

The script asks `yes/no`. Only `yes` deletes both named volumes.

## Faucet

After sync, a human can request signet coins from a public faucet and pay them
to an address created by a separate wallet. This directory does not do that.

## Troubleshooting

- `status.py` says bitcoind is unavailable: `docker compose ps` and
  `docker compose logs bitcoind`.
- Chain is not `signet`: stop and do not continue.
- electrs restarts: `docker compose logs electrs`. The usual cause is bitcoind
  not healthy yet; Compose waits for the healthcheck before starting electrs.
- Docker prints a `docker-sbom` plugin warning. Report it. Do not edit
  `~/.docker`.
