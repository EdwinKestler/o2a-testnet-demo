#!/usr/bin/env python3
"""Run the block-0 runbook on public signet with a stand-in artist.

The fresh seed stays in /tmp/o2a-artist-device and is never written here.
"""

import json
import os
import re
import secrets
import shutil
import subprocess
import sys
import time
from pathlib import Path

os.environ["DOCKER_CONTEXT"] = "default"

ROOT = Path("/media/kestl/andor/ffwd/o2a-testnet-demo")
PROTOCOL = Path("/media/kestl/andor/ffwd/o2a-protocol")
EVIDENCE = ROOT / "evidence/signet-block0-rehearsal-2026-09-28"
RAW = EVIDENCE / "raw"
PUBLIC = EVIDENCE / "public"
ARTIST = Path("/tmp/o2a-artist-device")
PAPER = Path("/tmp/o2a-artist-paper")
STAFF = Path("/tmp/o2a-staff")
SEED = ARTIST / "seed.hex"
ARTIST_PKG = ARTIST / "package"
OPERATOR = Path("/tmp/o2a-operator")
VER_A = Path("/tmp/o2a-verifier-a")
VER_B = Path("/tmp/o2a-verifier-b")
BACKUP = Path("/tmp/o2a-backup-operator")
RESTORE = Path("/tmp/o2a-restore")
ACK = "--ack-disposable"
ENTITY = "1"
DELAY = "1008"
AMOUNT = "0.0001"
NAME = "Rehearsal Name"
ELECTRUM = "tcp://signet-infra-electrs-1:60601"


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


def fields(text):
    found = {}
    for line in text.splitlines():
        if "=" in line and not line.lstrip().startswith("{") and not line.lstrip().startswith("\""):
            key, value = line.split("=", 1)
            if " " not in key and key.replace("_", "").replace(".", "").isalnum():
                found[key] = value.strip()
    return found


def btc(*args, name, check=True):
    return run(
        [
            "docker", "compose", "-p", "signet-infra",
            "--file", "dev/signet/docker-compose.yml",
            "--file", "dev/signet/compose.wallet.yaml",
            "exec", "-T", "bitcoind", "bitcoin-cli", "-signet", "-datadir=/data",
            *args,
        ],
        name,
        check,
    )


def btc_json(*args, name):
    text, _ = btc(*args, name=name)
    start = min(index for index in (text.find("{"), text.find("[")) if index >= 0)
    return json.loads(text[start:])


def demo(args, name, delay=DELAY, depth="1", seed=False, mounts=None, check=True):
    cmd = [
        "docker", "compose", "-p", "o2a-testnet-demo",
        "--file", "dev/compose.yaml",
        "--file", "dev/compose.signet-route.yaml",
        "--profile", "tools", "run", "--rm",
        "-e", f"O2A_DEMO_ENTITY={ENTITY}",
        "-e", "O2A_DEMO_NETWORK=signet",
        "-e", f"O2A_DEMO_DELAY={delay}",
        "-e", "O2A_DEMO_THRESHOLD=2",
        "-e", f"O2A_DEMO_DEPTH={depth}",
        "-v", f"{PROTOCOL}:/o2a-protocol:ro",
    ]
    if seed:
        cmd += ["-e", "O2A_DEMO_SEED_FILE=/run/seed", "-v", f"{SEED}:/run/seed:ro"]
    for host, guest in mounts or []:
        cmd += ["-v", f"{host}:{guest}"]
    cmd += ["toolchain", "target/debug/o2a-demo", *args]
    return run(cmd, name, check)


def timed(label, function):
    start = time.perf_counter()
    value = function()
    elapsed = time.perf_counter() - start
    with (RAW / "timings.txt").open("a") as handle:
        handle.write(f"{label}={elapsed:.3f}\n")
    print(f"TIMING {label} {elapsed:.3f}s", flush=True)
    return value


def make_seed():
    ARTIST.mkdir(mode=0o700, exist_ok=True)
    PAPER.mkdir(mode=0o700, exist_ok=True)
    STAFF.mkdir(mode=0o700, exist_ok=True)
    (STAFF / "README.txt").write_text("O2A staff hold no recovery share for this rehearsal.\n")
    source = (ROOT / "crates/o2a-demo-core/src/lib.rs").read_text()
    published = "".join(re.findall(r'"([0-9a-f]{64})"', source.split("UNSAFE_BIP39_SEED_HEX", 1)[1][:400]))
    if SEED.exists():
        raw = bytes.fromhex(SEED.read_text().strip())
    else:
        raw = secrets.token_bytes(64)
        SEED.write_text(raw.hex() + "\n")
        os.chmod(SEED, 0o600)
    if raw.hex() == published or len(raw) != 64:
        raise SystemExit("refusing the published unsafe seed")
    (RAW / "seed-location.txt").write_text(
        "artist_seed=/tmp/o2a-artist-device/seed.hex\n"
        "paper_share=/tmp/o2a-artist-paper/recovery-2.hex\n"
        "staff_share=none\n"
        "seed_in_git=false\n"
    )


def paper_share():
    if (PAPER / "recovery-2.hex").exists():
        return
    code = r"""
import sys
sys.path.insert(0, "/media/kestl/andor/ffwd/o2a-protocol/tests/vectors")
import derive_route_b as route
seed = bytes.fromhex(open("/tmp/o2a-artist-device/seed.hex").read().strip())
master = route.bip32_master(seed)
o2a = route.bip85_xprv(master, route.O2A_INDEX)
key = route.derive_path(o2a, [route.hardened(1), route.hardened(1), route.hardened(2), route.hardened(2)])
open("/tmp/o2a-artist-paper/recovery-2.hex", "w").write(key.encode() + "\n")
print(route.xonly_pub(key.secret))
"""
    text, _ = run(["python3", "-c", code], "paper-xonly")
    os.chmod(PAPER / "recovery-2.hex", 0o600)
    (RAW / "paper-xonly.txt").write_text(text)


def electrs_height():
    script = (
        "exec 3<>/dev/tcp/electrs/60601; "
        "printf '%s\\n' '{\"id\":1,\"method\":\"blockchain.headers.subscribe\",\"params\":[]}' >&3; "
        "timeout 3 cat <&3 || true"
    )
    text, _ = run(
        ["docker", "exec", "-i", "signet-infra-bitcoind-1", "bash", "-lc", script],
        "electrs-height",
        check=False,
    )
    match = re.search(r'"height":\s*(\d+)', text)
    if match is None:
        raise SystemExit("signet electrs did not answer")
    return int(match.group(1))


def toolchain_electrs():
    run(
        [
            "docker", "compose", "-p", "o2a-testnet-demo",
            "--file", "dev/compose.yaml", "--file", "dev/compose.signet-route.yaml",
            "--profile", "tools", "run", "--rm", "toolchain",
            "bash", "-lc",
            "timeout 5 bash -c 'echo >/dev/tcp/signet-infra-electrs-1/60601' && echo toolchain_electrs=open",
        ],
        "toolchain-electrs",
    )


def plan(delay, name):
    text, _ = demo(["seal", "plan", ACK], name, delay=delay, seed=True)
    found = fields(text)
    if not found.get("address", "").startswith("tb1"):
        raise SystemExit(f"{name} did not produce a signet address")
    return found


def core_match(descriptor, address, name):
    info = btc_json("getdescriptorinfo", descriptor, name=f"{name}-descriptor")
    derived = btc_json("deriveaddresses", info["descriptor"], name=f"{name}-derive")
    (RAW / f"{name}-core.txt").write_text(
        f"address={address}\nderived={derived[0]}\nchecksum={info['checksum']}\n"
    )
    if derived[0] != address:
        raise SystemExit(f"{name} core address mismatch")


def fund(address):
    text, _ = btc(
        "-rpcwallet=rehearsal", "-named", "sendtoaddress",
        f"address={address}", f"amount={AMOUNT}", "replaceable=false",
        name="fund-send",
    )
    txid = text.strip().splitlines()[-1]
    decoded = btc_json("getrawtransaction", txid, "1", name="fund-tx")
    sequences = [item["sequence"] for item in decoded["vin"]]
    if any(sequence < 4294967294 for sequence in sequences):
        raise SystemExit(f"funding transaction signals replacement: {sequences}")
    vout = next(item["n"] for item in decoded["vout"] if item["scriptPubKey"].get("address") == address)
    amount = next(item["value"] for item in decoded["vout"] if item["n"] == vout)
    (RAW / "fund.txt").write_text(
        f"txid={txid}\nvout={vout}\namount={amount}\nsequences={sequences}\n"
    )
    return txid, vout


def wait_depth(txid):
    while True:
        decoded = btc_json("getrawtransaction", txid, "1", name="fund-depth")
        conf = int(decoded.get("confirmations", 0))
        print(f"CONFIRMATIONS {conf}", flush=True)
        if conf >= 6:
            (RAW / "fund-final.txt").write_text(
                f"txid={txid}\nconfirmations={conf}\nblockhash={decoded['blockhash']}\n"
            )
            header = btc_json("getblockheader", decoded["blockhash"], name="fund-header")
            return conf, int(header["height"])
        time.sleep(30)


def copy_public(src, dest):
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True)
    for relative in ("genesis.o2a", "public.txt", "claim.o2a", "genesis.rgb", "lineage.txt"):
        path = src / relative
        if path.exists():
            shutil.copy2(path, dest / relative)
    if (src / "seals").exists():
        shutil.copytree(src / "seals", dest / "seals")


def verify_pair(label):
    for who, dest in (("a", VER_A), ("b", VER_B)):
        demo(
            ["seal", "verify", ELECTRUM, "/data", "A", "/data/validator"],
            f"{label}-{who}",
            depth="6",
            seed=False,
            mounts=[(dest, "/data")],
        )
    left = fields((RAW / f"{label}-a.txt").read_text())
    right = fields((RAW / f"{label}-b.txt").read_text())
    keys = ("entity_id", "identity_history_state", "best_height", "rgb")
    same = all(left.get(key) == right.get(key) for key in keys)
    (RAW / f"{label}-identical.txt").write_text(
        "byte_identical=" + ("true" if same else "false") + "\n"
        + "\n".join(f"{key}={left.get(key, '')}" for key in keys)
        + "\n"
    )
    if not same or left.get("identity_history_state") != "CURRENT":
        raise SystemExit(f"{label} verifiers did not both report CURRENT")
    return left


def main():
    RAW.mkdir(parents=True, exist_ok=True)
    (RAW / "timings.txt").write_text("")
    make_seed()
    paper_share()
    local = int(btc("getblockcount", name="node-height")[0].strip().splitlines()[-1])
    public = subprocess.check_output(
        ["curl", "-fsS", "--max-time", "20", "https://blockstream.info/signet/api/blocks/tip/height"],
        text=True,
    ).strip()
    indexed = electrs_height()
    toolchain_electrs()
    (RAW / "heights.txt").write_text(f"node={local}\nblockstream={public}\nelectrs={indexed}\n")
    if str(local) != public or indexed != local:
        raise SystemExit("signet heights do not match")

    real = timed("plan", lambda: plan(DELAY, "plan"))
    wrong = timed("wrong-policy", lambda: plan("10", "wrong-policy"))
    if real["address"] == wrong["address"]:
        raise SystemExit("wrong delay produced the same seal address")
    (RAW / "wrong-policy-result.txt").write_text(
        f"real_delay=1008\nreal_address={real['address']}\n"
        f"wrong_delay=10\nwrong_address={wrong['address']}\nmatched=false\n"
    )
    core_match(real["descriptor"], real["address"], "plan")

    if not (RAW / "fund.txt").exists():
        timed("fund", lambda: fund(real["address"]))
    funded = fields((RAW / "fund.txt").read_text())
    conf, height = timed("wait-depth", lambda: wait_depth(funded["txid"]))
    print(f"DEPTH_REACHED {conf} at {height}", flush=True)

    outpoint = f"{funded['txid']}:{funded['vout']}"
    if ARTIST_PKG.exists():
        shutil.rmtree(ARTIST_PKG)
    ARTIST_PKG.mkdir(mode=0o700)

    def sign():
        demo(
            ["seal", "sign-genesis", "/data", outpoint, ACK],
            "sign-genesis",
            seed=True,
            mounts=[(ARTIST_PKG, "/data")],
        )
    timed("sign-genesis", sign)
    signed = fields((RAW / "sign-genesis.txt").read_text())
    if signed.get("signer_entity_zero") != "true" or len(signed.get("entity_id", "")) != 64:
        raise SystemExit("genesis signature did not produce an EntityID")

    # Projector test page, then clear it before the real id.
    run(
        ["python3", "docs/block0-screen.py", "init", "00" * 32, "TEST ONLY"],
        "projector-test",
    )
    shutil.copy2("/tmp/block0-screen/index.html", RAW / "projector-test.html")
    shutil.rmtree("/tmp/block0-screen")

    if OPERATOR.exists():
        shutil.rmtree(OPERATOR)
    OPERATOR.mkdir()
    for name in ("genesis.o2a", "public.txt"):
        shutil.copy2(ARTIST_PKG / name, OPERATOR / name)

    def anchor():
        demo(
            ["seal", "record", "/data", "A", outpoint, funded["txid"], str(height), "10000", "genesis", ACK],
            "record",
            seed=True,
            mounts=[(OPERATOR, "/data")],
        )
        demo(
            ["seal", "issue", "/data", ELECTRUM, "A", ACK],
            "issue",
            seed=False,
            mounts=[(OPERATOR, "/data")],
        )
    timed("anchor", anchor)

    copy_public(OPERATOR, VER_A)
    copy_public(OPERATOR, VER_B)
    current = timed("verify", lambda: verify_pair("verify"))

    def show():
        run(["python3", "docs/block0-screen.py", "init", signed["entity_id"], NAME], "screen-init")
        run(
            ["python3", "docs/block0-screen.py", "verifier", "A", "CURRENT", current["best_height"]],
            "screen-a",
        )
        run(
            ["python3", "docs/block0-screen.py", "verifier", "B", "CURRENT", current["best_height"]],
            "screen-b",
        )
    timed("projector", show)

    def claim():
        demo(
            ["seal", "sign-claim", "/data", NAME, ACK],
            "sign-claim",
            seed=True,
            mounts=[(ARTIST_PKG, "/data")],
        )
    timed("sign-claim", claim)
    shutil.copy2(ARTIST_PKG / "claim.o2a", OPERATOR / "claim.o2a")
    shutil.copy2(ARTIST_PKG / "claim.o2a", VER_A / "claim.o2a")
    demo(
        ["seal", "verify-claim", "/data"],
        "verify-claim",
        seed=False,
        mounts=[(VER_A, "/data")],
    )
    timed("screen-claim", lambda: run(
        ["python3", "docs/block0-screen.py", "claim", NAME],
        "screen-claim",
    ))
    page = Path("/tmp/block0-screen/index.html").read_text()
    if signed["entity_id"] not in page or "Name claim signed:" not in page or SEED.read_text().strip() in page:
        raise SystemExit("projector page failed the public-line check")
    shutil.copy2("/tmp/block0-screen/index.html", RAW / "projector.html")

    copy_public(OPERATOR, BACKUP)
    shutil.copy2(ARTIST_PKG / "claim.o2a", BACKUP / "claim.o2a")
    copy_public(BACKUP, RESTORE)
    def restore():
        demo(
            ["seal", "verify", ELECTRUM, "/data", "A", "/data/validator"],
            "restore-verify",
            depth="6",
            seed=False,
            mounts=[(RESTORE, "/data")],
        )
        demo(
            ["seal", "verify-claim", "/data"],
            "restore-claim",
            seed=False,
            mounts=[(RESTORE, "/data")],
        )
    timed("restore", restore)
    restored = fields((RAW / "restore-verify.txt").read_text())
    if restored.get("entity_id") != signed["entity_id"] or restored.get("identity_history_state") != "CURRENT":
        raise SystemExit("restore did not reproduce the EntityID")

    demo(
        ["seal", "verify", "tcp://127.0.0.1:9", "/data", "A"],
        "network-down",
        depth="6",
        seed=False,
        mounts=[(VER_B, "/data")],
        check=False,
    )
    if "CURRENT" in fields((RAW / "network-down.txt").read_text()):
        raise SystemExit("offline verifier reported CURRENT")

    copy_public(OPERATOR, PUBLIC)
    shutil.copy2(ARTIST_PKG / "claim.o2a", PUBLIC / "claim.o2a")
    (RAW / "public-plan.txt").write_text((RAW / "plan.txt").read_text())
    print("STAGE_DONE", flush=True)


def continue_stage():
    """Resume after the seal record was rewritten with the artist policy."""
    funded = fields((RAW / "fund.txt").read_text())
    signed = fields((RAW / "sign-genesis.txt").read_text())
    height = "324067"
    outpoint = f"{funded['txid']}:{funded['vout']}"
    seal_file = OPERATOR / "seals" / "A.txt"
    if seal_file.exists():
        seal_file.unlink()
    demo(
        ["seal", "record", "/data", "A", outpoint, funded["txid"], height, "10000", "genesis", ACK],
        "record",
        seed=True,
        mounts=[(OPERATOR, "/data")],
    )
    recorded = (OPERATOR / "seals" / "A.txt").read_text()
    plan_script = fields((RAW / "plan.txt").read_text())["script_pubkey"]
    if f"script_pubkey={plan_script}" not in recorded:
        raise SystemExit("rerecorded seal script does not match the funded address")
    genesis = (OPERATOR / "genesis.o2a").read_text()
    policy = fields(recorded)["policy_hex"]
    if policy not in genesis:
        raise SystemExit("artist seal policy is not inside the signed genesis")
    copy_public(OPERATOR, VER_A)
    copy_public(OPERATOR, VER_B)
    for dest in (VER_A, VER_B):
        validator = dest / "validator"
        if validator.exists():
            shutil.rmtree(validator)
    current = timed("verify", lambda: verify_pair("verify"))

    def show():
        run(["python3", "docs/block0-screen.py", "init", signed["entity_id"], NAME], "screen-init")
        run(
            ["python3", "docs/block0-screen.py", "verifier", "A", "CURRENT", current["best_height"]],
            "screen-a",
        )
        run(
            ["python3", "docs/block0-screen.py", "verifier", "B", "CURRENT", current["best_height"]],
            "screen-b",
        )
    timed("projector", show)

    def claim():
        demo(
            ["seal", "sign-claim", "/data", NAME, ACK],
            "sign-claim",
            seed=True,
            mounts=[(ARTIST_PKG, "/data")],
        )
    timed("sign-claim", claim)
    for dest in (OPERATOR, VER_A, VER_B):
        shutil.copy2(ARTIST_PKG / "claim.o2a", dest / "claim.o2a")
    demo(["seal", "verify-claim", "/data"], "verify-claim", seed=False, mounts=[(VER_A, "/data")])
    timed("screen-claim", lambda: run(
        ["python3", "docs/block0-screen.py", "claim", NAME], "screen-claim",
    ))
    page = Path("/tmp/block0-screen/index.html").read_text()
    if signed["entity_id"] not in page or "Name claim signed:" not in page or "xprv" in page.lower():
        raise SystemExit("projector page failed the public-line check")
    shutil.copy2("/tmp/block0-screen/index.html", RAW / "projector.html")
    copy_public(OPERATOR, BACKUP)
    shutil.copy2(ARTIST_PKG / "claim.o2a", BACKUP / "claim.o2a")
    copy_public(BACKUP, RESTORE)

    def restore():
        demo(
            ["seal", "verify", ELECTRUM, "/data", "A", "/data/validator"],
            "restore-verify", depth="6", seed=False, mounts=[(RESTORE, "/data")],
        )
        demo(
            ["seal", "verify-claim", "/data"],
            "restore-claim", seed=False, mounts=[(RESTORE, "/data")],
        )
    timed("restore", restore)
    restored = fields((RAW / "restore-verify.txt").read_text())
    if restored.get("entity_id") != signed["entity_id"] or restored.get("identity_history_state") != "CURRENT":
        raise SystemExit("restore did not reproduce the EntityID")
    demo(
        ["seal", "verify", "tcp://127.0.0.1:9", "/data", "A"],
        "network-down", depth="6", seed=False, mounts=[(VER_B, "/data")], check=False,
    )
    if fields((RAW / "network-down.txt").read_text()).get("identity_history_state") == "CURRENT":
        raise SystemExit("offline verifier reported CURRENT")
    copy_public(OPERATOR, PUBLIC)
    shutil.copy2(ARTIST_PKG / "claim.o2a", PUBLIC / "claim.o2a")
    (RAW / "public-plan.txt").write_text((RAW / "plan.txt").read_text())
    print("STAGE_DONE", flush=True)


if __name__ == "__main__":
    try:
        if "--continue" in sys.argv:
            continue_stage()
        else:
            main()
    except SystemExit as error:
        print(f"REHEARSAL_FAIL {error}", flush=True)
        raise
