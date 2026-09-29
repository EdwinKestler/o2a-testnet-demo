//! Regtest stage commands with the seal funded outside the binary.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/rgb011-check")
}
const RPC_USER: &str = "o2ae2e";
const RPC_PASSWORD: &str = "o2ae2e-pass";

struct Cleanup {
    network: String,
    bitcoin: String,
    electrs: String,
    evidence: PathBuf,
    seed: PathBuf,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = docker(&["rm", "-f", &self.electrs]);
        let _ = docker(&["rm", "-f", &self.bitcoin]);
        let _ = docker(&["network", "rm", &self.network]);
        let _ = std::fs::remove_dir_all(&self.evidence);
        let _ = std::fs::remove_file(&self.seed);
    }
}

fn docker(args: &[&str]) -> Output {
    Command::new("docker")
        .env("DOCKER_CONTEXT", "default")
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("docker {}: {err}", args.join(" ")))
}

fn show(output: &Output) -> String {
    format!(
        "status {}\nstdout {}\nstderr {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn docker_ok(args: &[&str]) -> String {
    let output = docker(args);
    if !output.status.success() {
        panic!("docker {} failed\n{}", args.join(" "), show(&output));
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn cli(container: &str, args: &[&str]) -> Output {
    // This image's bitcoin-cli accepts -rpcuser=<user> and -rpcpassword=<pw> only.
    let user = format!("-rpcuser={RPC_USER}");
    let password = format!("-rpcpassword={RPC_PASSWORD}");
    let mut owned = vec![
        "exec".to_string(),
        container.to_string(),
        "bitcoin-cli".to_string(),
        "-regtest".to_string(),
        "-datadir=/data".to_string(),
        "-rpcconnect=127.0.0.1".to_string(),
        "-rpcport=18443".to_string(),
        user,
        password,
    ];
    owned.extend(args.iter().map(|arg| (*arg).to_string()));
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    docker(&refs)
}

fn cli_ok(container: &str, args: &[&str]) -> String {
    let output = cli(container, args);
    if !output.status.success() {
        panic!("bitcoin-cli {} failed\n{}", args.join(" "), show(&output));
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn published_port(container: &str, internal: u16) -> String {
    let text = docker_ok(&["port", container, &format!("{internal}/tcp")]);
    text.rsplit(':').next().unwrap_or("").trim().to_string()
}

fn electrum_height(addr: &str) -> Option<u64> {
    let mut stream = TcpStream::connect(addr).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .ok()?;
    let line = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"blockchain.headers.subscribe\",\"params\":[]}\n";
    stream.write_all(line.as_bytes()).ok()?;
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = stream.read(&mut byte).ok()?;
        if read == 0 || byte[0] == b'\n' {
            break;
        }
        buf.push(byte[0]);
        if buf.len() > 1_000_000 {
            return None;
        }
    }
    let text = String::from_utf8(buf).ok()?;
    let key = "\"height\":";
    let pos = text.find(key)?;
    let rest = text[pos + key.len()..].trim_start();
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn container_logs(name: &str) -> String {
    let output = docker(&["logs", "--tail", "80", name]);
    format!(
        "logs stdout:\n{}\nlogs stderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn wait_until(
    label: &str,
    container: &str,
    seconds: u64,
    mut probe: impl FnMut() -> Result<(), String>,
) {
    let start = Instant::now();
    let mut last = String::from("no attempt");
    while start.elapsed() < Duration::from_secs(seconds) {
        let inspect = docker(&[
            "inspect",
            "-f",
            "{{.State.Status}} exit={{.State.ExitCode}}",
            container,
        ]);
        let state = String::from_utf8_lossy(&inspect.stdout).trim().to_string();
        if inspect.status.success() && !state.starts_with("running") {
            panic!(
                "{label} container {container} is {state}\n{}\n{last}",
                container_logs(container)
            );
        }
        match probe() {
            Ok(()) => return,
            Err(err) => last = err,
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!(
        "{label} was not ready after {seconds}s\n{last}\n{}",
        container_logs(container)
    );
}

fn field(text: &str, key: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing {key} in {text}"))
        .to_string()
}

fn vout_for_script(json: &str, script_hex: &str) -> u32 {
    let spaced = format!("\"hex\": \"{script_hex}\"");
    let compact = format!("\"hex\":\"{script_hex}\"");
    let pos = json
        .find(&spaced)
        .or_else(|| json.find(&compact))
        .unwrap_or_else(|| panic!("script {script_hex} missing in {json}"));
    let before = &json[..pos];
    let npos = before
        .rfind("\"n\":")
        .unwrap_or_else(|| panic!("vout n missing before script"));
    let rest = before[npos + 4..]
        .trim_start()
        .trim_start_matches(':')
        .trim_start();
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("vout {digits} in {json}"))
}

fn run_tool(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command.env_clear().args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    command
        .output()
        .unwrap_or_else(|err| panic!("rgb011-check {}: {err}", args.join(" ")))
}

fn run_tool_stdin(args: &[&str], env: &[(&str, &str)], stdin_text: &str) -> Output {
    let mut command = Command::new(bin());
    command
        .env_clear()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .unwrap_or_else(|err| panic!("rgb011-check {}: {err}", args.join(" ")));
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin_text.as_bytes())
        .expect("stdin write");
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("rgb011-check {}: {err}", args.join(" ")))
}

fn assert_no_printed_address(output: &Output) {
    let out = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(!out.contains("address="), "{out}");
    assert!(!out.contains("bc1"), "{out}");
    assert!(
        !err.contains("preview uses the published unsafe seed"),
        "{err}"
    );
}

fn tool_ok(args: &[&str], env: &[(&str, &str)]) -> String {
    let output = run_tool(args, env);
    if !output.status.success() {
        panic!("rgb011-check {} failed\n{}", args.join(" "), show(&output));
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn deprecated_alias_sets_no_network() {
    let output = run_tool(&["signet-genesis"], &[("O2A_NETWORK", "regtest")]);
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{err}");
    assert!(err.contains("deprecated"), "{err}");
    assert!(err.contains("sets no network"), "{err}");
    assert!(err.contains("O2A_DEMO_SEED_FILE is required"), "{err}");
    assert!(!err.contains("must be signet"), "{err}");
}

#[test]
fn mainnet_plan_without_authorization_is_refused_before_keys() {
    let output = run_tool(
        &["plan"],
        &[("O2A_NETWORK", "mainnet"), ("O2A_DEMO_UNSAFE_PREVIEW", "1")],
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{err}");
    assert!(err.contains("--authorize-mainnet"), "{err}");
    assert_no_printed_address(&output);
}

#[test]
fn authorized_mainnet_plan_without_a_seed_file_prints_no_address() {
    let output = run_tool_stdin(
        &["plan", "--authorize-mainnet"],
        &[("O2A_NETWORK", "mainnet"), ("O2A_DEMO_UNSAFE_PREVIEW", "1")],
        "mainnet\n",
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{err}");
    assert!(err.contains("O2A_DEMO_SEED_FILE is required"), "{err}");
    assert!(
        err.contains("type mainnet to confirm this plan session"),
        "{err}"
    );
    assert_no_printed_address(&output);
}

#[test]
fn unsafe_preview_runs_only_on_regtest_or_signet_when_requested() {
    let refused = run_tool(&["plan"], &[("O2A_NETWORK", "regtest")]);
    let refused_err = String::from_utf8_lossy(&refused.stderr);
    assert!(!refused.status.success(), "{refused_err}");
    assert!(
        refused_err.contains("O2A_DEMO_SEED_FILE is required"),
        "{refused_err}"
    );
    assert_no_printed_address(&refused);

    let regtest = run_tool(
        &["plan"],
        &[("O2A_NETWORK", "regtest"), ("O2A_DEMO_UNSAFE_PREVIEW", "1")],
    );
    let regtest_err = String::from_utf8_lossy(&regtest.stderr);
    let regtest_out = String::from_utf8_lossy(&regtest.stdout);
    assert!(regtest.status.success(), "{regtest_err}\n{regtest_out}");
    assert!(
        regtest_err.contains("preview uses the published unsafe seed"),
        "{regtest_err}"
    );
    assert!(regtest_out.contains("address=bcrt1p"), "{regtest_out}");

    let signet = run_tool(
        &["plan"],
        &[("O2A_NETWORK", "signet"), ("O2A_DEMO_UNSAFE_PREVIEW", "1")],
    );
    let signet_out = String::from_utf8_lossy(&signet.stdout);
    let signet_err = String::from_utf8_lossy(&signet.stderr);
    assert!(signet.status.success(), "{signet_err}\n{signet_out}");
    assert!(
        signet_err.contains("preview uses the published unsafe seed"),
        "{signet_err}"
    );
    assert!(signet_out.contains("address=tb1p"), "{signet_out}");
}

#[test]
fn neutral_commands_use_external_funding_on_regtest() {
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis()
    );
    let network = format!("o2a-np-net-{suffix}");
    let bitcoin = format!("o2a-np-btc-{suffix}");
    let electrs = format!("o2a-np-el-{suffix}");
    let evidence = std::env::temp_dir().join(format!("o2a-np-evidence-{suffix}"));
    let seed = std::env::temp_dir().join(format!("o2a-np-seed-{suffix}"));
    std::fs::create_dir_all(&evidence).expect("evidence dir");
    std::fs::write(&seed, "42".repeat(64)).expect("seed");
    let _cleanup = Cleanup {
        network: network.clone(),
        bitcoin: bitcoin.clone(),
        electrs: electrs.clone(),
        evidence: evidence.clone(),
        seed: seed.clone(),
    };

    docker_ok(&["network", "create", &network]);
    docker_ok(&[
        "run",
        "-d",
        "--name",
        &bitcoin,
        "--network",
        &network,
        "-p",
        "127.0.0.1::18443",
        "-e",
        "BITCOIN_DATA=/data",
        "bitcoin/bitcoin:31.1",
        "-regtest=1",
        "-server=1",
        "-txindex=1",
        "-rest=1",
        "-listen=0",
        "-discover=0",
        "-natpmp=0",
        "-dnsseed=0",
        "-walletrbf=0",
        "-fallbackfee=0.0002",
        &format!("-rpcuser={RPC_USER}"),
        &format!("-rpcpassword={RPC_PASSWORD}"),
        "-rpcbind=0.0.0.0:18443",
        "-rpcallowip=0.0.0.0/0",
        "-printtoconsole=1",
    ]);
    let bitcoin_name = bitcoin.clone();
    wait_until("bitcoind", &bitcoin_name, 90, || {
        let output = cli(&bitcoin, &["getblockchaininfo"]);
        if output.status.success() {
            Ok(())
        } else {
            Err(show(&output))
        }
    });
    cli_ok(&bitcoin, &["createwallet", "miner"]);
    let mine_to = cli_ok(&bitcoin, &["-rpcwallet=miner", "getnewaddress"])
        .trim_matches('"')
        .to_string();
    cli_ok(
        &bitcoin,
        &["-rpcwallet=miner", "generatetoaddress", "101", &mine_to],
    );

    let electrs_script = format!(
        r#"set -e
mkdir -p "$HOME/db"
cat > "$HOME/electrs.toml" <<EOF
network = "regtest"
db_dir = "$HOME/db"
daemon_rpc_addr = "{bitcoin}:18443"
electrum_rpc_addr = "0.0.0.0:50001"
auth = "{RPC_USER}:{RPC_PASSWORD}"
log_filters = "info"
EOF
exec electrs --skip-default-conf-files --conf "$HOME/electrs.toml"
"#
    );
    docker_ok(&[
        "run",
        "-d",
        "--name",
        &electrs,
        "--network",
        &network,
        "-p",
        "127.0.0.1::50001",
        "--entrypoint",
        "/bin/sh",
        "getumbrel/electrs:v0.12.0",
        "-c",
        &electrs_script,
    ]);
    let rpc_port = published_port(&bitcoin, 18443);
    let electrum_port = published_port(&electrs, 50001);
    let electrum_addr = format!("127.0.0.1:{electrum_port}");
    let electrs_name = electrs.clone();
    wait_until("electrs", &electrs_name, 180, || {
        match electrum_height(&electrum_addr) {
            Some(height) if height >= 100 => Ok(()),
            Some(height) => Err(format!("electrs height {height}")),
            None => Err("electrum did not answer".into()),
        }
    });

    let common = [
        ("O2A_NETWORK", "regtest".to_string()),
        ("RGB_CHAIN", "regtest".to_string()),
        ("O2A_DEMO_SEED_FILE", seed.display().to_string()),
        ("O2A_DEMO_ENTITY", "9001".to_string()),
        ("O2A_DEMO_DELAY", "10".to_string()),
        ("O2A_DEMO_THRESHOLD", "2".to_string()),
        ("BITCOIN_RPC", format!("http://127.0.0.1:{rpc_port}")),
        ("BITCOIN_RPC_USER", RPC_USER.to_string()),
        ("BITCOIN_RPC_PASSWORD", RPC_PASSWORD.to_string()),
        ("ELECTRUM", electrum_addr.clone()),
        ("BITCOIN_WALLET", "miner".to_string()),
        ("RGB011_EVIDENCE", evidence.display().to_string()),
    ];
    let env: Vec<(&str, &str)> = common.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let plan = tool_ok(&["plan"], &env);
    let address = field(&plan, "address");
    let policy = field(&plan, "policy");
    let script = field(&plan, "script_pubkey");
    assert!(address.starts_with("bcrt1p"), "{address}");
    assert!(policy.starts_with("tr("), "{policy}");
    assert!(!script.is_empty());

    let txid = cli_ok(
        &bitcoin,
        &[
            "-rpcwallet=miner",
            "-named",
            "sendtoaddress",
            &format!("address={address}"),
            "amount=0.002",
            "replaceable=false",
        ],
    )
    .trim_matches('"')
    .to_string();
    let change = cli_ok(&bitcoin, &["-rpcwallet=miner", "getnewaddress"])
        .trim_matches('"')
        .to_string();
    cli_ok(
        &bitcoin,
        &["-rpcwallet=miner", "generatetoaddress", "1", &change],
    );
    let raw = cli_ok(&bitcoin, &["getrawtransaction", &txid, "1"]);
    let vout = vout_for_script(&raw, &script);
    assert!(
        raw.contains("\"sequence\": 4294967294")
            || raw.contains("\"sequence\":4294967294")
            || raw.contains("\"sequence\": 4294967295")
            || raw.contains("\"sequence\":4294967295"),
        "funding sequences are not final\n{raw}"
    );
    assert!(
        raw.contains("\"confirmations\": 1") || raw.contains("\"confirmations\":1"),
        "{raw}"
    );
    wait_until(
        "electrs funding block",
        &electrs,
        90,
        || match electrum_height(&electrum_addr) {
            Some(height) if height >= 101 => Ok(()),
            Some(height) => Err(format!("electrs height {height}")),
            None => Err("electrum did not answer".into()),
        },
    );

    let seal = format!("{txid}:{vout}");
    let preflight = tool_ok(&["preflight", "--json", "--seal", &seal], &env);
    assert!(
        preflight.contains(&format!("\"seal_address\": \"{address}\"")),
        "{preflight}"
    );
    assert!(
        preflight.contains("\"funding_confirmations\": 1"),
        "{preflight}"
    );
    assert!(preflight.contains("\"unspent\": true"), "{preflight}");
    assert!(evidence.join("preflight.json").is_file());

    let genesis = tool_ok(&["genesis", "--seal", &seal], &env);
    assert!(genesis.contains("entity_id="), "{genesis}");
    assert!(genesis.contains("confirmations=1"), "{genesis}");
    assert!(evidence.join("genesis.o2a").is_file());
    assert!(evidence.join("genesis.strict").is_file());
    assert!(evidence.join("public.txt").is_file());

    let mut claim_env = common.to_vec();
    claim_env.push(("O2A_REHEARSAL_NAME", "Regtest Neutral Vector".to_string()));
    let claim_pairs: Vec<(&str, &str)> = claim_env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let claim = tool_ok(&["claim"], &claim_pairs);
    assert!(
        claim.contains("name_claim=Regtest Neutral Vector"),
        "{claim}"
    );

    let verify_env: Vec<(&str, &str)> = common
        .iter()
        .filter(|(key, _)| *key != "O2A_DEMO_SEED_FILE" && *key != "O2A_DEMO_ENTITY")
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let verified = tool_ok(&["verify"], &verify_env);
    assert!(
        verified.contains("identity_history_state=CURRENT"),
        "{verified}"
    );
    assert!(verified.contains("name_claim=valid"), "{verified}");
}
