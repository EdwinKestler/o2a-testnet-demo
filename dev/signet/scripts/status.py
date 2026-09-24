#!/usr/bin/env python3
"""Print signet bitcoind and electrs status. Exit 0 only when both tips match and bitcoind is synced."""

import json
import socket
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RPC_URL = "http://127.0.0.1:38332/"
ELECTRUM_ADDR = ("127.0.0.1", 60601)


def load_env(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key] = value
    return values


def rpc(user: str, password: str, method: str) -> dict:
    body = json.dumps({"jsonrpc": "1.0", "id": "status", "method": method, "params": []}).encode()
    request = urllib.request.Request(RPC_URL, data=body, method="POST")
    token = f"{user}:{password}".encode()
    request.add_header("Authorization", "Basic " + __import__("base64").b64encode(token).decode())
    request.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(request, timeout=30) as response:
        payload = json.load(response)
    if payload.get("error"):
        raise SystemExit(f"bitcoind RPC error: {payload['error']}")
    return payload["result"]


def electrum_call(sock: socket.socket, method: str, params: list) -> dict:
    message = {"id": 0, "method": method, "params": params}
    sock.sendall((json.dumps(message) + "\n").encode())
    data = b""
    while b"\n" not in data:
        chunk = sock.recv(65536)
        if not chunk:
            break
        data += chunk
    return json.loads(data.decode().split("\n", 1)[0])


def electrs_height() -> int:
    with socket.create_connection(ELECTRUM_ADDR, timeout=30) as sock:
        version = electrum_call(sock, "server.version", ["signet-infra", "1.4"])
        if "error" in version and version["error"]:
            raise SystemExit(f"electrs server.version error: {version['error']}")
        tip = electrum_call(sock, "blockchain.headers.subscribe", [])
        if tip.get("error"):
            raise SystemExit(f"electrs headers.subscribe error: {tip['error']}")
        result = tip["result"]
        if isinstance(result, dict) and "height" in result:
            return int(result["height"])
        if isinstance(result, list) and result:
            return int(result[0].get("height"))
        raise SystemExit(f"unexpected electrs tip: {result}")


def main() -> int:
    env = load_env(ROOT / ".env")
    info = rpc(env["RPC_USER"], env["RPC_PASSWORD"], "getblockchaininfo")
    chain = info["chain"]
    blocks = int(info["blocks"])
    headers = int(info["headers"])
    progress = info["verificationprogress"]
    ibd = bool(info["initialblockdownload"])
    best = info["bestblockhash"]
    try:
        indexed = electrs_height()
    except (OSError, TimeoutError, json.JSONDecodeError) as error:
        print(f"electrs: unavailable ({error})")
        indexed = None
    print(f"chain {chain}")
    print(f"headers {headers}")
    print(f"blocks {blocks}")
    print(f"verificationprogress {progress}")
    print(f"initialblockdownload {str(ibd).lower()}")
    print(f"bestblockhash {best}")
    print(f"electrs_height {indexed if indexed is not None else 'unavailable'}")
    synced = chain == "signet" and not ibd and blocks == headers and blocks > 0
    if synced and indexed == blocks:
        return 0
    return 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except urllib.error.URLError as error:
        print(f"bitcoind: unavailable ({error})")
        sys.exit(1)
