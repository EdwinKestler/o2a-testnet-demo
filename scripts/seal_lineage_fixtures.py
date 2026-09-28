#!/usr/bin/env python3
"""Disposable regtest fixtures for the seal-policy lineage. Not a Phase 0 gate."""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

os.environ["DOCKER_CONTEXT"] = "default"

ROOT = Path("/media/kestl/andor/ffwd/o2a-testnet-demo")
EVIDENCE = ROOT / "evidence/regtest-seal-policy-lineage-2026-09-26"
RAW = EVIDENCE / "raw"
WORK = EVIDENCE / "work"
ACK = "--ack-disposable"
ELECTRUM = "tcp://electrs:50001"
H0_FLOOR = 104


def run(cmd, name, check=True):
    print("+", " ".join(cmd), flush=True)
    result = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True)
    text = result.stdout + result.stderr
    RAW.mkdir(parents=True, exist_ok=True)
    (RAW / f"{name}.txt").write_text(text)
    print(text, flush=True)
    if check and result.returncode != 0:
        raise SystemExit(f"{name} failed with {result.returncode}")
    return text, result.returncode


def btc(*args, name, check=True):
    return run(
        [
            "docker",
            "compose",
            "-p",
            "o2a-testnet-demo",
            "--file",
            "dev/compose.yaml",
            "exec",
            "-T",
            "bitcoin",
            "bitcoin-cli",
            "-regtest",
            "-datadir=/var/lib/bitcoin",
            "-rpcwallet=evidence",
            *args,
        ],
        name,
        check,
    )


def btc_json(*args, name, check=True):
    text, code = btc(*args, name=name, check=check)
    start_obj = text.find("{")
    start_arr = text.find("[")
    starts = [pos for pos in (start_obj, start_arr) if pos >= 0]
    if not starts:
        raise SystemExit(f"{name} returned no JSON\n{text}")
    return json.loads(text[min(starts) :]), code


def container_path(value):
    text = str(value)
    host = str(ROOT)
    if text.startswith(host):
        return "/workspace" + text[len(host) :]
    return text


def cli(*args, name, check=True):
    mapped = [container_path(arg) for arg in args]
    return run(
        [
            "docker",
            "compose",
            "-p",
            "o2a-testnet-demo",
            "--file",
            "dev/compose.yaml",
            "--profile",
            "tools",
            "run",
            "--rm",
            "-v",
            "/media/kestl/andor/ffwd/o2a-protocol:/o2a-protocol:ro",
            "toolchain",
            "target/debug/o2a-demo",
            *mapped,
        ],
        name,
        check,
    )


def field(text, key):
    for line in text.splitlines():
        if line.startswith(key + "="):
            return line.split("=", 1)[1]
    raise SystemExit(f"missing {key}")


def report_lines(text):
    lines = []
    started = False
    for line in text.splitlines():
        if line.startswith("best_height="):
            started = True
        if started:
            lines.append(line)
            if line.startswith("header_trust="):
                break
    return "\n".join(lines) + "\n"


def chain_height(name):
    text, _ = btc("getblockcount", name=name)
    return int(text.strip().splitlines()[-1])


def electrs_height(name):
    script = (
        "exec 3<>/dev/tcp/electrs/50001; "
        'printf "%s\\n" \'{"id":1,"method":"blockchain.headers.subscribe","params":[]}\' >&3; '
        "IFS= read -r line <&3; printf '%s\\n' \"$line\""
    )
    text, _ = run(
        [
            "docker",
            "compose",
            "-p",
            "o2a-testnet-demo",
            "--file",
            "dev/compose.yaml",
            "exec",
            "-T",
            "bitcoin",
            "bash",
            "-lc",
            script,
        ],
        name,
    )
    for line in text.splitlines():
        if line.startswith("{"):
            return int(json.loads(line)["result"]["height"])
    raise SystemExit(f"electrs height missing in {name}")


def wait_electrs(name):
    target = chain_height(f"{name}-bitcoind")
    for _ in range(40):
        height = electrs_height(f"{name}-electrs")
        if height == target:
            return height
        time.sleep(1)
    raise SystemExit(f"electrs stayed behind bitcoind for {name}")


def mine(count, name):
    btc("generatetoaddress", str(count), miner, name=name)
    return wait_electrs(name)


def fund(address, name):
    text, _ = btc("sendtoaddress", address, "0.001", name=f"{name}-send")
    txid = text.strip().splitlines()[-1]
    mine(1, f"{name}-mine")
    decoded, _ = btc_json("getrawtransaction", txid, "1", name=f"{name}-tx")
    vout = next(item["n"] for item in decoded["vout"] if item["scriptPubKey"].get("address") == address)
    output = decoded["vout"][vout]
    height = chain_height(f"{name}-height")
    script = output["scriptPubKey"]["hex"]
    return txid, f"{txid}:{vout}", height, script


def fresh_script(name):
    text, _ = btc("getnewaddress", name=f"{name}-address")
    address = text.strip().splitlines()[-1]
    info, _ = btc_json("getaddressinfo", address, name=f"{name}-addressinfo")
    script = info.get("scriptPubKey")
    if not script:
        raise SystemExit(f"{name} address info has no scriptPubKey")
    return address, script


def cross_check(data, stage, name):
    text, _ = cli("seal", "prepare", str(data), stage, ACK, name=f"{name}-prepare")
    address = field(text, "address")
    descriptor = field(text, "descriptor")
    info, _ = btc_json("getdescriptorinfo", descriptor, name=f"{name}-getdescriptorinfo")
    derived, _ = btc_json("deriveaddresses", info["descriptor"], name=f"{name}-deriveaddresses")
    (RAW / f"{name}-core-crosscheck.txt").write_text(
        "address={address}\nderived={derived}\nchecksum={checksum}\ndescriptor={descriptor}\n".format(
            address=address,
            derived=derived[0],
            checksum=info["checksum"],
            descriptor=info["descriptor"],
        )
    )
    if derived[0] != address:
        raise SystemExit(f"{name} core address mismatch")
    return address


def record(data, seal_name, outpoint, txid, height, stage, name):
    cli(
        "seal",
        "record",
        str(data),
        seal_name,
        outpoint,
        txid,
        str(height),
        "100000",
        stage,
        ACK,
        name=name,
    )


def fee_input(data, name):
    text, _ = cli("seal", "fee-address", str(data), name=f"{name}-fee-address")
    address = field(text, "fee_address")
    txid, outpoint, _height, script = fund(address, f"{name}-fee")
    return outpoint, "100000", script


def transition(data, from_name, to_outpoint, fee, fee_value, fee_script, kind, name):
    text, _ = cli(
        "seal",
        "transition",
        str(data),
        ELECTRUM,
        from_name,
        to_outpoint,
        fee,
        fee_value,
        fee_script,
        kind,
        "broadcast",
        ACK,
        name=name,
    )
    mine(1, f"{name}-mine")
    return text


def verify_pair(data, seal_name, label, flag=None):
    left = WORK / f"{label}-a"
    right = WORK / f"{label}-b"
    args = ["seal", "verify", ELECTRUM, str(data), seal_name]
    if flag:
        cli(*args, str(left), flag, name=f"{label}-a")
        cli(*args, str(right), flag, name=f"{label}-b")
    else:
        cli(*args, str(left), name=f"{label}-a")
        cli(*args, str(right), name=f"{label}-b")
    report_a = report_lines((RAW / f"{label}-a.txt").read_text())
    report_b = report_lines((RAW / f"{label}-b.txt").read_text())
    (RAW / f"{label}-identical.txt").write_text(
        f"byte_identical={str(report_a == report_b).lower()}\n{report_a}"
    )
    if report_a != report_b:
        raise SystemExit(f"{label} validator outputs differ")
    return report_a


def require_state(report, state, label):
    if f"identity_history_state={state}" not in report:
        raise SystemExit(f"{label} expected {state}\n{report}")


def main():
    global miner
    RAW.mkdir(parents=True, exist_ok=True)
    stage = sys.argv[1] if len(sys.argv) > 1 else "all"
    text, _ = btc("getnewaddress", name="miner-address")
    miner = text.strip().splitlines()[-1]
    h0 = int((RAW / "H0").read_text().strip())
    if h0 != H0_FLOOR:
        raise SystemExit(f"recorded H0 is {h0}")
    if chain_height("height-now") < h0:
        raise SystemExit("chain is below the recorded H0")

    id1 = WORK / "id1"
    if stage in {"all", "f2"}:
        cross_check(id1, "rotate", "f2")
        address = field((RAW / "f2-prepare.txt").read_text(), "address")
        txid, outpoint, height, _script = fund(address, "f2-fund")
        record(id1, "B", outpoint, txid, height, "rotate", "f2-record")
        fee, fee_value, fee_script = fee_input(id1, "f2")
        transition(id1, "A", outpoint, fee, fee_value, fee_script, "rotate", "f2-rotate")
        report = verify_pair(id1, "B", "f2-verify")
        require_state(report, "CURRENT", "f2")
        cli("seal", "stale", name="f2-stale")
        print("F2 done", flush=True)
        if stage == "f2":
            return

    if stage in {"all", "f3"}:
        address = field((RAW / "f2-prepare.txt").read_text(), "address")
        txid, outpoint, height, _script = fund(address, "f3-fund-c")
        record(id1, "C", outpoint, txid, height, "rotate", "f3-record-c")
        _dest_address, dest_script = fresh_script("f3-dest")
        text, _ = cli(
            "seal",
            "presign-recovery",
            str(id1),
            "B",
            dest_script,
            ACK,
            name="f3-presign",
        )
        raw = field(text, "raw_tx")
        rejected, code = btc("sendrawtransaction", raw, name="f3-early-send", check=False)
        (RAW / "f3-early-status.txt").write_text(f"exit={code}\n{rejected}")
        if code == 0 or "non-BIP68-final" not in rejected:
            raise SystemExit("early recovery was not rejected as non-BIP68-final")
        fee, fee_value, fee_script = fee_input(id1, "f3")
        transition(id1, "B", outpoint, fee, fee_value, fee_script, "rotate-1", "f3-rotate")
        spent, spent_code = btc("sendrawtransaction", raw, name="f3-spent-send", check=False)
        (RAW / "f3-spent-status.txt").write_text(f"exit={spent_code}\n{spent}")
        if spent_code == 0:
            raise SystemExit("prebuilt recovery was accepted after the input was spent")
        report = verify_pair(id1, "C", "f3-verify")
        require_state(report, "CURRENT", "f3")
        print("F3 done", flush=True)
        if stage == "f3":
            return

    if stage in {"all", "f4"}:
        address = field((RAW / "f2-prepare.txt").read_text(), "address")
        txid, outpoint, height, _script = fund(address, "f4-fund-d")
        record(id1, "D", outpoint, txid, height, "rotate", "f4-record-d")
        c_height = int(
            (id1 / "seals" / "C.txt").read_text().split("confirmation_height=")[1].splitlines()[0]
        )
        target = c_height + 10
        now = chain_height("f4-before-delay")
        if now < target:
            mine(target - now, "f4-delay")
        if chain_height("f4-delay-height") < target:
            raise SystemExit("delay height was not reached")
        fee, fee_value, fee_script = fee_input(id1, "f4")
        transition(id1, "C", outpoint, fee, fee_value, fee_script, "recover", "f4-recover")
        report = verify_pair(id1, "D", "f4-verify")
        require_state(report, "CURRENT", "f4")
        offline = verify_pair(id1, "D", "f8-verify", flag="--no-observation")
        require_state(offline, "INCOMPLETE", "f8")
        print("F4 and F8 done", flush=True)
        if stage == "f4":
            return

    id2 = WORK / "id2"
    if stage in {"all", "f5"}:
        address = cross_check(id2, "genesis", "f5-genesis")
        txid, outpoint, height, _script = fund(address, "f5-fund-a")
        record(id2, "A", outpoint, txid, height, "genesis", "f5-record-a")
        cli("seal", "issue", str(id2), ELECTRUM, "A", ACK, name="f5-issue")
        cli("seal", "wallet", str(id2), ELECTRUM, outpoint, name="f5-wallet")
        rotate_address = cross_check(id2, "rotate", "f5-rotate-prepare")
        txid, next_out, height, _script = fund(rotate_address, "f5-fund-b")
        record(id2, "B", next_out, txid, height, "rotate", "f5-record-b")
        fee, fee_value, fee_script = fee_input(id2, "f5")
        transition(id2, "A", next_out, fee, fee_value, fee_script, "rotate", "f5-rotate")
        before = verify_pair(id2, "B", "f5-before")
        require_state(before, "CURRENT", "f5-before")
        anchor = field((id2 / "lineage.txt").read_text(), "anchor_txid")
        decoded, _ = btc_json("getrawtransaction", anchor, "1", name="f5-anchor-tx")
        block_hash = decoded["blockhash"]
        header, _ = btc_json("getblockheader", block_hash, name="f5-anchor-header")
        if int(header["height"]) <= h0:
            raise SystemExit("refusing to invalidate a block at or below H0")
        (RAW / "f5-anchor-height.txt").write_text(
            f"anchor_txid={anchor}\nblockhash={block_hash}\nheight={header['height']}\nh0={h0}\n"
        )
        btc("invalidateblock", block_hash, name="f5-invalidate")
        wait_electrs("f5-after-invalidate")
        missing = verify_pair(id2, "B", "f5-reorg")
        require_state(missing, "INCOMPLETE", "f5-reorg")
        btc("reconsiderblock", block_hash, name="f5-reconsider")
        wait_electrs("f5-after-reconsider")
        restored = verify_pair(id2, "B", "f5-restored")
        require_state(restored, "CURRENT", "f5-restored")
        print("F5 done", flush=True)
        if stage == "f5":
            return

    if stage in {"all", "f6"}:
        _dest_address, dest_script = fresh_script("f6-dest")
        text, _ = cli(
            "seal",
            "close-plain",
            str(id2),
            "B",
            dest_script,
            ACK,
            name="f6-close",
        )
        raw = field(text, "raw_tx")
        btc("sendrawtransaction", raw, name="f6-send")
        mine(1, "f6-mine")
        closed = verify_pair(id2, "B", "f6-verify")
        require_state(closed, "SEAL_CLOSED_WITHOUT_VALID_TRANSITION", "f6")
        print("F6 done", flush=True)

    if stage in {"all", "f7"}:
        id3 = WORK / "id3"
        id3.mkdir(parents=True, exist_ok=True)
        _address, script = fresh_script("f7-address")
        text, _ = btc("sendtoaddress", _address, "0.001", name="f7-send")
        txid = text.strip().splitlines()[-1]
        mine(1, "f7-mine")
        decoded, _ = btc_json("getrawtransaction", txid, "1", name="f7-tx")
        vout = next(item["n"] for item in decoded["vout"] if item["scriptPubKey"].get("address") == _address)
        height = chain_height("f7-height")
        record(id3, "A", f"{txid}:{vout}", txid, height, "genesis", "f7-record")
        cli("seal", "mismatch", f"{txid}:{vout}", name="f7-mismatch")
        report = verify_pair(id3, "A", "f7-verify")
        require_state(report, "INVALID", "f7")
        print("F7 done", flush=True)


if __name__ == "__main__":
    main()
