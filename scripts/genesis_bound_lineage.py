#!/usr/bin/env python3
"""Regtest lineage for genesis-bound EntityIDs. Not a Phase 0 gate."""

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

os.environ["DOCKER_CONTEXT"] = "default"

ROOT = Path("/media/kestl/andor/ffwd/o2a-testnet-demo")
EVIDENCE = ROOT / "evidence/regtest-genesis-bound-lineage-2026-09-28"
RAW = EVIDENCE / "raw"
WORK = EVIDENCE / "work"
ACK = "--ack-disposable"
ELECTRUM = "tcp://electrs:50001"
PROTOCOL = "/media/kestl/andor/ffwd/o2a-protocol:/o2a-protocol:ro"


def run(cmd, name, check=True, env=None):
    print("+", " ".join(cmd), flush=True)
    result = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, env=env)
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
    start = min(pos for pos in (text.find("{"), text.find("[")) if pos >= 0)
    return json.loads(text[start:]), code


def container_path(value):
    text = str(value)
    host = str(ROOT)
    if text.startswith(host):
        return "/workspace" + text[len(host) :]
    return text


def cli(*args, name, entity=None, depth=None, check=True):
    command = [
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
    ]
    if entity is not None:
        command += ["-e", f"O2A_DEMO_ENTITY={entity}"]
    if depth is not None:
        command += ["-e", f"O2A_DEMO_DEPTH={depth}"]
    command += [
        "-v",
        PROTOCOL,
        "toolchain",
        "target/debug/o2a-demo",
        *[container_path(arg) for arg in args],
    ]
    return run(command, name, check)


def field(text, key):
    for line in text.splitlines():
        if line.startswith(key + "="):
            return line.split("=", 1)[1]
    raise SystemExit(f"missing {key}")


def report_lines(text):
    lines = []
    started = False
    for line in text.splitlines():
        if line.startswith("entity_id=") or line.startswith("entity_index=") or line.startswith("best_height="):
            started = True
        if started:
            lines.append(line)
            if line.startswith("header_trust="):
                break
    return "\n".join(lines) + "\n"


def chain_height(name):
    text, _ = btc("getblockcount", name=name)
    return int(text.strip().splitlines()[-1])


def mine(count, name, miner):
    btc("generatetoaddress", str(count), miner, name=name)
    return chain_height(f"{name}-height")


def fund(address, name, miner, amount="0.001", replaceable=False):
    if replaceable:
        text, _ = btc(
            "-named",
            "sendtoaddress",
            f"address={address}",
            f"amount={amount}",
            "replaceable=true",
            name=f"{name}-send",
        )
    else:
        text, _ = btc(
            "-named",
            "sendtoaddress",
            f"address={address}",
            f"amount={amount}",
            "replaceable=false",
            name=f"{name}-send",
        )
    txid = text.strip().splitlines()[-1]
    return txid


def outpoint_for(txid, address, name):
    decoded, _ = btc_json("getrawtransaction", txid, "1", name=name)
    vout = next(item["n"] for item in decoded["vout"] if item["scriptPubKey"].get("address") == address)
    return f"{txid}:{vout}", decoded


def confirmations(txid, name):
    try:
        decoded, code = btc_json("getrawtransaction", txid, "1", name=name, check=False)
    except Exception:
        return None
    if code != 0:
        return None
    return int(decoded.get("confirmations", 0))


def probe_electrs_height():
    script = (
        "exec 3<>/dev/tcp/electrs/50001; "
        "printf '%s\\n' '{\"id\":1,\"method\":\"blockchain.headers.subscribe\",\"params\":[]}' >&3; "
        "timeout 3 cat <&3 || true"
    )
    result = subprocess.run(
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
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    match = re.search(r'"height":\s*(\d+)', result.stdout + result.stderr)
    if match is None:
        return None
    return int(match.group(1))


def wait_electrs(label):
    target = chain_height(f"{label}-chain")
    last = None
    for _ in range(30):
        last = probe_electrs_height()
        if last is not None and last >= target:
            (RAW / f"{label}-electrs.txt").write_text(
                f"electrs_height={last}\nbitcoind_height={target}\n"
            )
            return
        time.sleep(1)
    raise SystemExit(f"{label} electrs height {last} is below bitcoind {target}")


def verify_pair(data, seal_name, label, entity, depth=1, flag=None):
    wait_electrs(label)
    left = WORK / f"{label}-a"
    right = WORK / f"{label}-b"
    args = ["seal", "verify", ELECTRUM, str(data), seal_name]
    extra = [flag] if flag else []
    cli(*args, str(left), *extra, name=f"{label}-a", entity=entity, depth=depth)
    cli(*args, str(right), *extra, name=f"{label}-b", entity=entity, depth=depth)
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


def cross_check(data, stage, name, entity):
    text, _ = cli("seal", "prepare", str(data), stage, ACK, name=f"{name}-prepare", entity=entity)
    address = field(text, "address")
    descriptor = field(text, "descriptor")
    info, _ = btc_json("getdescriptorinfo", descriptor, name=f"{name}-descriptor")
    derived, _ = btc_json("deriveaddresses", info["descriptor"], name=f"{name}-derive")
    (RAW / f"{name}-core.txt").write_text(
        f"address={address}\nderived={derived[0]}\nchecksum={info['checksum']}\n"
    )
    if derived[0] != address:
        raise SystemExit(f"{name} core address mismatch")
    return address


def main():
    RAW.mkdir(parents=True, exist_ok=True)
    WORK.mkdir(parents=True, exist_ok=True)
    miner = btc("getnewaddress", name="miner")[0].strip().splitlines()[-1]
    h0 = chain_height("h0")
    (RAW / "H0").write_text(f"{h0}\n")

    # Identity entity 1: pending, current, rotation, recovery, same-seal fork.
    entity = 1
    data = WORK / "e1"
    address = cross_check(data, "genesis", "e1", entity)
    txid = fund(address, "e1-fund", miner)
    mine(1, "e1-fund-mine", miner)
    outpoint, _decoded = outpoint_for(txid, address, "e1-fund-tx")
    height = chain_height("e1-fund-height")
    cli(
        "seal", "record", str(data), "A", outpoint, txid, str(height), "100000", "genesis", ACK,
        name="e1-record", entity=entity,
    )
    issued, _ = cli("seal", "issue", str(data), ELECTRUM, "A", ACK, name="e1-issue", entity=entity)
    legit_id = field(issued, "entity_id")
    pending = verify_pair(data, "A", "e1-pending", entity, depth=6)
    require_state(pending, "PENDING_CONFIRMATION", "e1-pending")
    need = 6 - confirmations(txid, "e1-conf-before")
    if need > 0:
        mine(need, "e1-to-depth", miner)
    current = verify_pair(data, "A", "e1-current", entity, depth=6)
    require_state(current, "CURRENT", "e1-current")
    if field(current, "entity_id") != legit_id:
        raise SystemExit("current entity id changed")

    rotate_address = cross_check(data, "rotate", "e1-rotate", entity)
    btx = fund(rotate_address, "e1-bfund", miner)
    mine(1, "e1-bfund-mine", miner)
    bout, _ = outpoint_for(btx, rotate_address, "e1-btx")
    bheight = chain_height("e1-bheight")
    cli(
        "seal", "record", str(data), "B", bout, btx, str(bheight), "100000", "rotate", ACK,
        name="e1-brecord", entity=entity,
    )
    fee_text, _ = cli("seal", "fee-address", str(data), name="e1-fee-address", entity=entity)
    fee_address = field(fee_text, "fee_address")
    ftx = fund(fee_address, "e1-fee", miner)
    mine(1, "e1-fee-mine", miner)
    fdec, _ = btc_json("getrawtransaction", ftx, "1", name="e1-fee-tx")
    fvout = next(item for item in fdec["vout"] if item["scriptPubKey"].get("address") == fee_address)
    rotated, _ = cli(
        "seal", "transition", str(data), ELECTRUM, "A", bout,
        f"{ftx}:{fvout['n']}", "100000", fvout["scriptPubKey"]["hex"],
        "rotate", "broadcast", ACK,
        name="e1-rotate-tx", entity=entity,
    )
    if field(rotated, "history_entity") != legit_id:
        raise SystemExit("rotation changed the EntityID")
    mine(1, "e1-rotate-mine", miner)
    after = verify_pair(data, "B", "e1-after-rotate", entity, depth=1)
    require_state(after, "CURRENT", "e1-after-rotate")

    # Same root, same seal A, different controller bytes.
    fork_text, _ = cli("seal", "fork", outpoint, name="e1-fork-ids", entity=entity)
    if field(fork_text, "ids_differ") != "true":
        raise SystemExit("fork entity id matched the legitimate id")
    if field(fork_text, "legit_entity_id") != legit_id:
        raise SystemExit("fork comparison used a different legitimate id")
    fork_dir = WORK / "e1-fork"
    cli(
        "seal", "record", str(fork_dir), "A", outpoint, txid, str(height), "100000", "genesis", ACK,
        name="e1-fork-record", entity=entity,
    )
    cli("seal", "fork-write", str(fork_dir), outpoint, name="e1-fork-write", entity=entity)
    (fork_dir / "lineage.txt").write_text(
        f"anchor_txid={txid}\nconsignment={container_path(data / 'rotate.rgb')}\nseal={outpoint}\n"
    )
    closed = verify_pair(fork_dir, "A", "e1-fork-closed", entity, depth=1)
    require_state(closed, "SEAL_CLOSED_WITHOUT_VALID_TRANSITION", "e1-fork")
    if field(closed, "entity_id") == legit_id:
        raise SystemExit("fork verifier reported the legitimate EntityID")

    # Recovery keeps the EntityID. Delay is 10 blocks from B's confirmation.
    target = bheight + 10
    now = chain_height("e1-before-recovery")
    if now < target:
        mine(target - now, "e1-recovery-delay", miner)
    dtx = fund(rotate_address, "e1-dfund", miner)
    mine(1, "e1-dfund-mine", miner)
    dout, _ = outpoint_for(dtx, rotate_address, "e1-dtx")
    dheight = chain_height("e1-dheight")
    cli(
        "seal", "record", str(data), "D", dout, dtx, str(dheight), "100000", "recover", ACK,
        name="e1-drecord", entity=entity,
    )
    fee_text, _ = cli("seal", "fee-address", str(data), name="e1-fee2-address", entity=entity)
    fee_address = field(fee_text, "fee_address")
    ftx = fund(fee_address, "e1-fee2", miner)
    mine(1, "e1-fee2-mine", miner)
    fdec, _ = btc_json("getrawtransaction", ftx, "1", name="e1-fee2-tx")
    fvout = next(item for item in fdec["vout"] if item["scriptPubKey"].get("address") == fee_address)
    recovered, _ = cli(
        "seal", "transition", str(data), ELECTRUM, "B", dout,
        f"{ftx}:{fvout['n']}", "100000", fvout["scriptPubKey"]["hex"],
        "recover", "broadcast", ACK,
        name="e1-recover", entity=entity,
    )
    if field(recovered, "history_entity") != legit_id:
        raise SystemExit("recovery changed the EntityID")
    mine(1, "e1-recover-mine", miner)
    recovered_view = verify_pair(data, "D", "e1-recovered", entity, depth=1)
    require_state(recovered_view, "CURRENT", "e1-recovered")

    offline = verify_pair(data, "D", "e1-no-observation", entity, depth=1, flag="--no-observation")
    require_state(offline, "INCOMPLETE", "e1-f8")

    # Identity entity 2: reorg of the seal-creating block, then plain close.
    entity = 2
    data2 = WORK / "e2"
    address2 = cross_check(data2, "genesis", "e2", entity)
    if address2 == address:
        raise SystemExit("entity 2 reused entity 1's seal address")
    tx2 = fund(address2, "e2-fund", miner)
    mine(1, "e2-mine", miner)
    op2, decoded2 = outpoint_for(tx2, address2, "e2-tx")
    block_hash = decoded2["blockhash"]
    header, _ = btc_json("getblockheader", block_hash, name="e2-header")
    if int(header["height"]) <= h0:
        raise SystemExit("refusing to invalidate a block at or below H0")
    h2 = chain_height("e2-height")
    cli(
        "seal", "record", str(data2), "A", op2, tx2, str(h2), "100000", "genesis", ACK,
        name="e2-record", entity=entity,
    )
    cli("seal", "issue", str(data2), ELECTRUM, "A", ACK, name="e2-issue", entity=entity)
    before = verify_pair(data2, "A", "e2-before", entity, depth=1)
    require_state(before, "CURRENT", "e2-before")
    btc("invalidateblock", block_hash, name="e2-invalidate")
    missing = verify_pair(data2, "A", "e2-reorg", entity, depth=1)
    require_state(missing, "INCOMPLETE", "e2-reorg")
    btc("reconsiderblock", block_hash, name="e2-reconsider")
    restored = verify_pair(data2, "A", "e2-restored", entity, depth=1)
    require_state(restored, "CURRENT", "e2-restored")
    dest_text, _code = btc("getnewaddress", name="e2-dest")
    finish_from_e2_close(miner, dest_text.strip().splitlines()[-1])


def last_line(name):
    return (RAW / f"{name}.txt").read_text().strip().splitlines()[-1]


def finish_from_e2_close(miner, dest_address):
    """F6, F7, RBF, and CPFP. The caller has already restored entity 2."""
    entity = 2
    data2 = WORK / "e2"
    info, _ = btc_json("getaddressinfo", dest_address, name="e2-dest-info")
    closed_text, _ = cli(
        "seal", "close-plain", str(data2), "A", info["scriptPubKey"], ACK,
        name="e2-close", entity=entity,
    )
    btc("sendrawtransaction", field(closed_text, "raw_tx"), name="e2-close-send")
    mine(1, "e2-close-mine", miner)
    plain = verify_pair(data2, "A", "e2-plain", entity, depth=1)
    require_state(plain, "SEAL_CLOSED_WITHOUT_VALID_TRANSITION", "e2-f6")

    # Identity entity 3: script does not match the policy.
    entity = 3
    data3 = WORK / "e3"
    bad_address = btc("getnewaddress", name="e3-address")[0].strip().splitlines()[-1]
    bad_tx = fund(bad_address, "e3-fund", miner)
    mine(1, "e3-mine", miner)
    bad_op, _ = outpoint_for(bad_tx, bad_address, "e3-tx")
    bad_height = chain_height("e3-height")
    cli(
        "seal", "record", str(data3), "A", bad_op, bad_tx, str(bad_height), "100000", "genesis", ACK,
        name="e3-record", entity=entity,
    )
    mismatch = verify_pair(data3, "A", "e3-mismatch", entity, depth=1)
    require_state(mismatch, "INVALID", "e3-f7")
    finish_rbf_and_cpfp(miner)


def finish_rbf_and_cpfp(miner):
    # RBF replaces the outpoint. CPFP keeps it.
    entity = 4
    data4 = WORK / "e4"
    address4 = cross_check(data4, "genesis", "e4", entity)
    first = fund(address4, "e4-rbf", miner, replaceable=True)
    first_op, _ = outpoint_for(first, address4, "e4-rbf-tx")
    first_id = field(cli("seal", "fork", first_op, name="e4-id-before", entity=entity)[0], "legit_entity_id")
    bumped, _ = btc_json("bumpfee", first, name="e4-bump")
    second = bumped["txid"]
    second_op, _ = outpoint_for(second, address4, "e4-rbf-replacement")
    second_id = field(cli("seal", "fork", second_op, name="e4-id-after", entity=entity)[0], "legit_entity_id")
    if first_id == second_id:
        raise SystemExit("RBF kept the EntityID")
    if confirmations(first, "e4-old-tx") not in (None, 0):
        raise SystemExit("replaced funding transaction is still confirmed")
    (RAW / "e4-rbf-result.txt").write_text(
        f"replaced_txid={first}\nreplacement_txid={second}\nold_entity_id={first_id}\nnew_entity_id={second_id}\n"
    )

    parent = fund(address4, "e4-cpfp", miner, replaceable=False)
    parent_op, parent_tx = outpoint_for(parent, address4, "e4-cpfp-parent")
    parent_id = field(cli("seal", "fork", parent_op, name="e4-cpfp-id", entity=entity)[0], "legit_entity_id")
    change = next(item for item in parent_tx["vout"] if item["scriptPubKey"].get("address") != address4)
    child_address = btc("getnewaddress", name="e4-child-address")[0].strip().splitlines()[-1]
    btc(
        "-named", "send",
        f"outputs={{\"{child_address}\":0.0002}}",
        f"inputs=[{{\"txid\":\"{parent}\",\"vout\":{change['n']}}}]",
        name="e4-cpfp-child",
    )
    mine(1, "e4-cpfp-mine", miner)
    parent_after, _ = btc_json("getrawtransaction", parent, "1", name="e4-cpfp-after")
    if int(parent_after.get("confirmations", 0)) < 1:
        raise SystemExit("CPFP parent did not confirm")
    parent_id_after = field(
        cli("seal", "fork", parent_op, name="e4-cpfp-id-after", entity=entity)[0],
        "legit_entity_id",
    )
    if parent_id_after != parent_id:
        raise SystemExit("CPFP changed the EntityID")
    (RAW / "e4-cpfp-result.txt").write_text(
        f"parent_txid={parent}\nentity_id={parent_id}\nconfirmations={parent_after['confirmations']}\n"
    )
    print("lineage complete", flush=True)


def finish_from_e3(miner):
    """F7 was funded and recorded. Verify it, then run RBF and CPFP."""
    entity = 3
    data3 = WORK / "e3"
    mismatch = verify_pair(data3, "A", "e3-mismatch", entity, depth=1)
    require_state(mismatch, "INVALID", "e3-f7")
    finish_rbf_and_cpfp(miner)


if __name__ == "__main__":
    if "--resume-e2-close" in sys.argv:
        # The 2026-09-28 run stopped after entity 2 was reconsidered to CURRENT.
        # Continue without funding a second entity 1 or rewriting H0.
        recorded_h0 = int((RAW / "H0").read_text().strip())
        if recorded_h0 != 105:
            raise SystemExit(f"refusing to continue: H0 file is {recorded_h0}")
        finish_from_e2_close(last_line("miner"), last_line("e2-dest"))
    elif "--resume-e3" in sys.argv:
        recorded_h0 = int((RAW / "H0").read_text().strip())
        if recorded_h0 != 105:
            raise SystemExit(f"refusing to continue: H0 file is {recorded_h0}")
        finish_from_e3(last_line("miner"))
    else:
        main()
