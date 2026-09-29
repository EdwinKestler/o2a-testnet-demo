//! One network profile. Every other module reads this value and does not
//! keep a second network table.
//!
//! `genesis_sign_depth` stays 6 on every network. Bitcoin Core 28 can replace
//! a transaction that did not signal replacement, so `signet-genesis` keeps
//! the confirmation wait it already had. `required_depth` is the profile
//! depth from the network table.

use std::io::{self, Write};

use rgbstd::bitcoin::Network;
use rgbstd::ChainNet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkKind {
    Regtest,
    Signet,
    Mainnet,
}

impl NetworkKind {
    pub fn label(self) -> &'static str {
        match self {
            NetworkKind::Regtest => "regtest",
            NetworkKind::Signet => "signet",
            NetworkKind::Mainnet => "mainnet",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Plan,
    Genesis,
    OfficialName,
    Lineage,
    Verify,
    Transition,
    Broadcast,
    Compatibility,
}

impl Operation {
    pub fn name(self) -> &'static str {
        match self {
            Operation::Plan => "plan",
            Operation::Genesis => "genesis",
            Operation::OfficialName => "official_name",
            Operation::Lineage => "lineage",
            Operation::Verify => "verify",
            Operation::Transition => "transition",
            Operation::Broadcast => "broadcast",
            Operation::Compatibility => "compatibility",
        }
    }

    /// Local preview, genesis, and the one official-name claim.
    pub fn allowed_on_mainnet(self) -> bool {
        matches!(
            self,
            Operation::Plan | Operation::Genesis | Operation::OfficialName
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkProfile {
    pub kind: NetworkKind,
    pub network_byte: u8,
    pub coin_type: u32,
    pub hrp: &'static str,
    pub rgb_chain: ChainNet,
    pub bitcoin: Network,
    pub rpc_url: Option<String>,
    pub electrum: Option<String>,
    pub wallet: String,
    pub cookie_path: Option<String>,
    pub required_depth: u32,
    pub genesis_sign_depth: u32,
    pub create_miner_wallet: bool,
    pub offline: bool,
    pub identity: &'static str,
    pub regtest_requested: bool,
    pub signet_requested: bool,
    pub rgb_explicit_signet: bool,
    pub rgb_conflict: bool,
}

pub fn from_kind(kind: NetworkKind) -> NetworkProfile {
    let mainnet = kind == NetworkKind::Mainnet;
    let (
        network_byte,
        coin_type,
        hrp,
        rgb_chain,
        bitcoin,
        required_depth,
        wallet,
        rpc,
        electrum,
        cookie,
        create_miner,
    ) = match kind {
        NetworkKind::Regtest => (
            4,
            1,
            "bcrt",
            ChainNet::BitcoinRegtest,
            Network::Regtest,
            1,
            "miner",
            Some("http://127.0.0.1:18445".to_string()),
            Some("127.0.0.1:50021".to_string()),
            Some("/tmp/rgb011.cookie".to_string()),
            true,
        ),
        NetworkKind::Signet => (
            3,
            1,
            "tb",
            ChainNet::BitcoinSignet,
            Network::Signet,
            1,
            "rehearsal",
            Some("http://127.0.0.1:38332".to_string()),
            Some("127.0.0.1:60601".to_string()),
            Some("/tmp/rgb011.cookie".to_string()),
            false,
        ),
        NetworkKind::Mainnet => (
            0,
            0,
            "bc",
            ChainNet::BitcoinMainnet,
            Network::Bitcoin,
            6,
            "",
            None,
            None,
            None,
            false,
        ),
    };
    NetworkProfile {
        kind,
        network_byte,
        coin_type,
        hrp,
        rgb_chain,
        bitcoin,
        rpc_url: rpc,
        electrum,
        wallet: wallet.to_string(),
        cookie_path: cookie,
        required_depth,
        genesis_sign_depth: 6,
        create_miner_wallet: create_miner,
        offline: mainnet,
        identity: if mainnet { "permanent" } else { "disposable" },
        regtest_requested: false,
        signet_requested: false,
        rgb_explicit_signet: false,
        rgb_conflict: false,
    }
}

pub fn load() -> Result<NetworkProfile, String> {
    let selected = nonempty("O2A_NETWORK");
    let demo = nonempty("O2A_DEMO_NETWORK");
    let rgb = nonempty("RGB_CHAIN");
    let explicit = match (selected.as_deref(), demo.as_deref()) {
        (Some(left), Some(right)) => {
            let primary = parse_name(left)?;
            let fallback = parse_name(right)?;
            if primary != fallback {
                return Err(format!(
                    "O2A_NETWORK {left} does not match O2A_DEMO_NETWORK {right}"
                ));
            }
            Some(primary)
        }
        (Some(left), None) => Some(parse_name(left)?),
        (None, Some(right)) => Some(parse_name(right)?),
        (None, None) => None,
    };
    let rgb_kind = match rgb.as_deref() {
        Some(value) => Some(parse_name(value)?),
        None => None,
    };
    let (kind, rgb_conflict) = match (explicit, rgb_kind) {
        (Some(chosen), Some(chain)) if chosen != chain => (chosen, true),
        (Some(chosen), _) => (chosen, false),
        (None, Some(chain)) => (chain, false),
        (None, None) => (NetworkKind::Regtest, false),
    };
    let mut profile = from_kind(kind);
    profile.rgb_conflict = rgb_conflict;
    profile.regtest_requested = explicit == Some(NetworkKind::Regtest);
    profile.signet_requested = explicit == Some(NetworkKind::Signet);
    profile.rgb_explicit_signet = rgb_kind == Some(NetworkKind::Signet) && !rgb_conflict;
    if let Some(value) = nonempty("BITCOIN_RPC") {
        profile.rpc_url = Some(value);
    }
    if let Some(value) = nonempty("ELECTRUM") {
        profile.electrum = Some(value);
    }
    if let Some(value) = nonempty("BITCOIN_WALLET") {
        profile.wallet = value;
    }
    if let Some(value) = nonempty("BITCOIN_COOKIE") {
        profile.cookie_path = Some(value);
    }
    if profile.kind == NetworkKind::Mainnet {
        if nonempty("BITCOIN_RPC").is_none() {
            profile.rpc_url = None;
        }
        if nonempty("ELECTRUM").is_none() {
            profile.electrum = None;
        }
        if nonempty("BITCOIN_COOKIE").is_none() {
            profile.cookie_path = None;
        }
        if nonempty("BITCOIN_WALLET").is_none() {
            profile.wallet.clear();
        }
        profile.offline = true;
        profile.create_miner_wallet = false;
    }
    Ok(profile)
}

pub fn require_regtest_command(profile: &NetworkProfile) -> Result<(), String> {
    if !profile.regtest_requested || profile.kind != NetworkKind::Regtest {
        return Err("O2A_DEMO_NETWORK must be regtest".into());
    }
    if profile.rgb_conflict {
        return Err("RGB_CHAIN must be regtest".into());
    }
    Ok(())
}

pub fn require_signet_genesis(profile: &NetworkProfile) -> Result<(), String> {
    if !profile.signet_requested || profile.kind != NetworkKind::Signet {
        return Err("O2A_DEMO_NETWORK must be signet".into());
    }
    if !profile.rgb_explicit_signet {
        return Err("RGB_CHAIN must be signet".into());
    }
    Ok(())
}

pub fn require_signet_claim(profile: &NetworkProfile) -> Result<(), String> {
    if !profile.signet_requested || profile.kind != NetworkKind::Signet {
        return Err("O2A_DEMO_NETWORK must be signet".into());
    }
    Ok(())
}

pub fn require_signet_verify(profile: &NetworkProfile) -> Result<(), String> {
    if profile.rgb_conflict || !profile.rgb_explicit_signet || profile.kind != NetworkKind::Signet {
        return Err("RGB_CHAIN must be signet".into());
    }
    Ok(())
}

pub fn session_banner(profile: &NetworkProfile, operation: Operation) -> String {
    let mut text = format!(
        "active_network={}\nnetwork_byte={}\ncoin_type={}\naddress_prefix={}\nconfirmation_depth={}\nidentity={}\noperation={}\n",
        profile.kind.label(),
        profile.network_byte,
        profile.coin_type,
        profile.hrp,
        profile.required_depth,
        profile.identity,
        operation.name(),
    );
    if profile.kind == NetworkKind::Mainnet && operation.allowed_on_mainnet() {
        text.push_str(&format!(
            "type mainnet to confirm this {} session\n",
            operation.name()
        ));
    }
    text
}

pub fn decide(
    profile: &NetworkProfile,
    operation: Operation,
    authorize: bool,
    typed: Option<&str>,
) -> Result<(), String> {
    if profile.kind != NetworkKind::Mainnet {
        return Ok(());
    }
    if !operation.allowed_on_mainnet() {
        return Err(format!(
            "mainnet scope refuses {}; only the local plan, genesis, and the official_name claim are allowed",
            operation.name()
        ));
    }
    if !authorize {
        return Err("mainnet requires --authorize-mainnet for this session".into());
    }
    if typed.map(str::trim) != Some("mainnet") {
        return Err("mainnet confirmation was not the word mainnet".into());
    }
    Ok(())
}

pub fn begin_cli(
    profile: &NetworkProfile,
    operation: Operation,
    authorize: bool,
) -> Result<(), String> {
    let banner = session_banner(profile, operation);
    eprint!("{banner}");
    let _ = io::stderr().flush();
    let typed =
        if profile.kind == NetworkKind::Mainnet && operation.allowed_on_mainnet() && authorize {
            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .map_err(|err| err.to_string())?;
            Some(line)
        } else {
            None
        };
    decide(profile, operation, authorize, typed.as_deref())
}

pub fn allow_transport(profile: &NetworkProfile) -> Result<(), String> {
    if profile.offline || profile.kind == NetworkKind::Mainnet {
        return Err("network transport is disabled".into());
    }
    if profile.rpc_url.is_none() || profile.electrum.is_none() {
        return Err("backend endpoints are not configured".into());
    }
    Ok(())
}

pub fn refuse_broadcast(profile: &NetworkProfile) -> Result<(), String> {
    if profile.kind == NetworkKind::Mainnet {
        return Err("mainnet broadcast is refused".into());
    }
    Ok(())
}

pub fn is_broadcast_method(method: &str) -> bool {
    matches!(
        method,
        "sendrawtransaction" | "sendtoaddress" | "sendmany" | "submitpackage" | "submitblock"
    )
}

pub fn split_args(args: &[String]) -> Result<(Option<String>, bool), String> {
    let mut authorize = false;
    let mut command = None;
    for arg in args {
        if arg == "--authorize-mainnet" {
            authorize = true;
        } else if command.is_none() && !arg.starts_with('-') {
            command = Some(arg.clone());
        } else {
            return Err(format!("unknown argument {arg}"));
        }
    }
    Ok((command, authorize))
}

fn nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_name(value: &str) -> Result<NetworkKind, String> {
    match value {
        "regtest" => Ok(NetworkKind::Regtest),
        "signet" => Ok(NetworkKind::Signet),
        "mainnet" => Ok(NetworkKind::Mainnet),
        "testnet" | "testnet4" => {
            Err("testnet and testnet4 are not selectable runtime profiles".into())
        }
        other => Err(format!("unknown network {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use o2a_demo_core::{
        encode_resulting_state, entity_id, genesis_for, genesis_state_from, keys_for,
        official_name_claim, official_name_nonce, seal_for_state, state_id, verify,
    };
    use rgbstd::bitcoin::{Address, ScriptBuf};

    use super::*;

    static ENV: Mutex<()> = Mutex::new(());

    struct EnvLock {
        _guard: MutexGuard<'static, ()>,
        saved: Vec<(String, Option<String>)>,
    }

    impl Drop for EnvLock {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                unsafe {
                    match value {
                        Some(saved) => std::env::set_var(key, saved),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    fn lock_env(pairs: &[(&str, Option<&str>)]) -> EnvLock {
        let guard = ENV.lock().expect("env lock");
        let mut saved = Vec::new();
        for (key, value) in pairs {
            saved.push(((*key).to_string(), std::env::var(key).ok()));
            unsafe {
                match value {
                    Some(next) => std::env::set_var(key, next),
                    None => std::env::remove_var(key),
                }
            }
        }
        EnvLock {
            _guard: guard,
            saved,
        }
    }

    #[test]
    fn profile_table_matches_the_three_networks() {
        let regtest = from_kind(NetworkKind::Regtest);
        assert_eq!(regtest.network_byte, 4);
        assert_eq!(regtest.coin_type, 1);
        assert_eq!(regtest.hrp, "bcrt");
        assert_eq!(regtest.required_depth, 1);
        assert_eq!(regtest.rgb_chain, ChainNet::BitcoinRegtest);
        assert_eq!(regtest.bitcoin, Network::Regtest);
        assert_eq!(regtest.identity, "disposable");
        assert!(!regtest.offline);

        let signet = from_kind(NetworkKind::Signet);
        assert_eq!(signet.network_byte, 3);
        assert_eq!(signet.coin_type, 1);
        assert_eq!(signet.hrp, "tb");
        assert_eq!(signet.required_depth, 1);
        assert_eq!(signet.rgb_chain, ChainNet::BitcoinSignet);
        assert_eq!(signet.bitcoin, Network::Signet);
        assert_eq!(signet.rpc_url.as_deref(), Some("http://127.0.0.1:38332"));
        assert_eq!(signet.electrum.as_deref(), Some("127.0.0.1:60601"));
        assert_eq!(signet.identity, "disposable");

        let mainnet = from_kind(NetworkKind::Mainnet);
        assert_eq!(mainnet.network_byte, 0);
        assert_eq!(mainnet.coin_type, 0);
        assert_eq!(mainnet.hrp, "bc");
        assert_eq!(mainnet.required_depth, 6);
        assert_eq!(mainnet.rgb_chain, ChainNet::BitcoinMainnet);
        assert_eq!(mainnet.bitcoin, Network::Bitcoin);
        assert!(mainnet.rpc_url.is_none());
        assert!(mainnet.electrum.is_none());
        assert!(mainnet.offline);
        assert_eq!(mainnet.identity, "permanent");
        assert_eq!(mainnet.genesis_sign_depth, 6);
    }

    #[test]
    fn selectors_keep_the_regtest_and_signet_commands() {
        let _lock = lock_env(&[
            ("O2A_NETWORK", None),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", None),
            ("BITCOIN_RPC", None),
            ("ELECTRUM", None),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
        ]);
        let default_profile = load().expect("default");
        assert_eq!(default_profile.kind, NetworkKind::Regtest);
        assert!(!default_profile.regtest_requested);
        assert!(require_regtest_command(&default_profile).is_err());

        drop(_lock);
        let _lock = lock_env(&[
            ("O2A_NETWORK", None),
            ("O2A_DEMO_NETWORK", Some("regtest")),
            ("RGB_CHAIN", Some("regtest")),
            ("BITCOIN_RPC", None),
            ("ELECTRUM", None),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
        ]);
        let regtest = load().expect("regtest");
        assert_eq!(regtest.network_byte, 4);
        assert_eq!(regtest.rpc_url.as_deref(), Some("http://127.0.0.1:18445"));
        assert_eq!(regtest.electrum.as_deref(), Some("127.0.0.1:50021"));
        assert_eq!(regtest.wallet, "miner");
        require_regtest_command(&regtest).expect("lineage");

        drop(_lock);
        let _lock = lock_env(&[
            ("O2A_NETWORK", None),
            ("O2A_DEMO_NETWORK", Some("signet")),
            ("RGB_CHAIN", Some("signet")),
            ("BITCOIN_RPC", None),
            ("ELECTRUM", None),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
        ]);
        let signet = load().expect("signet");
        assert_eq!(signet.network_byte, 3);
        assert_eq!(signet.coin_type, 1);
        require_signet_genesis(&signet).expect("genesis");
        require_signet_claim(&signet).expect("claim");

        drop(_lock);
        let _lock = lock_env(&[
            ("O2A_NETWORK", None),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", Some("signet")),
            ("BITCOIN_RPC", None),
            ("ELECTRUM", None),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
        ]);
        let verify = load().expect("verify profile");
        assert_eq!(verify.kind, NetworkKind::Signet);
        assert_eq!(verify.network_byte, 3);
        require_signet_verify(&verify).expect("verify");
        assert!(require_signet_genesis(&verify).is_err());
        assert!(require_signet_claim(&verify).is_err());
    }

    #[test]
    fn mismatched_or_unknown_networks_are_errors() {
        let _lock = lock_env(&[
            ("O2A_NETWORK", Some("regtest")),
            ("O2A_DEMO_NETWORK", Some("signet")),
            ("RGB_CHAIN", None),
        ]);
        let err = load().expect_err("mismatch");
        assert!(err.contains("does not match"), "{err}");

        drop(_lock);
        let _lock = lock_env(&[
            ("O2A_NETWORK", Some("nope")),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", None),
        ]);
        let err = load().expect_err("unknown");
        assert!(err.contains("unknown network"), "{err}");

        drop(_lock);
        let _lock = lock_env(&[
            ("O2A_NETWORK", Some("testnet")),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", None),
        ]);
        let err = load().expect_err("testnet");
        assert!(err.contains("not selectable"), "{err}");

        drop(_lock);
        let _lock = lock_env(&[
            ("O2A_NETWORK", Some("regtest")),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", Some("signet")),
        ]);
        let profile = load().expect("conflict stays a profile");
        assert!(profile.rgb_conflict);
        let err = require_regtest_command(&profile).expect_err("chain");
        assert_eq!(err, "RGB_CHAIN must be regtest");
    }

    #[test]
    fn banners_name_the_active_network() {
        let regtest = session_banner(&from_kind(NetworkKind::Regtest), Operation::Plan);
        let signet = session_banner(&from_kind(NetworkKind::Signet), Operation::Genesis);
        let mainnet = session_banner(&from_kind(NetworkKind::Mainnet), Operation::Genesis);
        assert!(regtest.contains("active_network=regtest"));
        assert!(regtest.contains("identity=disposable"));
        assert!(!regtest.contains("mainnet"));
        assert!(signet.contains("active_network=signet"));
        assert!(signet.contains("identity=disposable"));
        assert!(signet.contains("operation=genesis"));
        assert!(mainnet.contains("active_network=mainnet"));
        assert!(mainnet.contains("identity=permanent"));
        assert!(mainnet.contains("confirmation_depth=6"));
        assert!(mainnet.contains("type mainnet to confirm this genesis session"));
        assert_ne!(regtest, signet);
        assert_ne!(signet, mainnet);
    }

    #[test]
    fn mainnet_lock_refuses_a_missing_word_and_transitions() {
        let mainnet = from_kind(NetworkKind::Mainnet);
        let missing = decide(&mainnet, Operation::Genesis, false, None).expect_err("flag");
        assert!(missing.contains("--authorize-mainnet"), "{missing}");
        let wrong = decide(&mainnet, Operation::Genesis, true, Some("regtest")).expect_err("word");
        assert!(wrong.contains("confirmation"), "{wrong}");
        decide(&mainnet, Operation::Genesis, true, Some("mainnet\n")).expect("genesis");
        decide(&mainnet, Operation::Plan, true, Some("mainnet")).expect("plan");
        decide(&mainnet, Operation::OfficialName, true, Some("mainnet")).expect("claim");
        let transition =
            decide(&mainnet, Operation::Transition, true, Some("mainnet")).expect_err("transition");
        assert!(transition.contains("scope"), "{transition}");
        let lineage =
            decide(&mainnet, Operation::Lineage, true, Some("mainnet")).expect_err("lineage");
        assert!(lineage.contains("scope"), "{lineage}");
        let broadcast =
            decide(&mainnet, Operation::Broadcast, true, Some("mainnet")).expect_err("broadcast");
        assert!(broadcast.contains("scope"), "{broadcast}");
        let refused = refuse_broadcast(&mainnet).expect_err("method");
        assert_eq!(refused, "mainnet broadcast is refused");
        decide(
            &from_kind(NetworkKind::Regtest),
            Operation::Lineage,
            false,
            None,
        )
        .expect("regtest");
        decide(
            &from_kind(NetworkKind::Signet),
            Operation::Genesis,
            false,
            None,
        )
        .expect("signet");
    }

    #[test]
    fn mainnet_transport_stays_closed_when_an_endpoint_is_set() {
        let _lock = lock_env(&[
            ("O2A_NETWORK", Some("mainnet")),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", None),
            ("BITCOIN_RPC", Some("http://127.0.0.1:1")),
            ("ELECTRUM", Some("127.0.0.1:1")),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
        ]);
        let profile = load().expect("mainnet");
        assert_eq!(profile.rpc_url.as_deref(), Some("http://127.0.0.1:1"));
        let err = match crate::chain::Node::connect_profile(&profile, false) {
            Err(err) => err,
            Ok(_) => panic!("network transport opened"),
        };
        assert_eq!(err, "network transport is disabled");
        assert!(!err.contains("rpc connect"));
    }

    #[test]
    fn offline_mainnet_dry_run_uses_mainnet_parameters_and_core() {
        let _lock = lock_env(&[
            ("O2A_DEMO_SEED_FILE", None),
            ("O2A_DEMO_ENTITY", None),
            ("O2A_DEMO_DELAY", None),
            ("O2A_DEMO_THRESHOLD", None),
            ("O2A_DEMO_NETWORK", None),
            ("O2A_NETWORK", None),
            ("RGB_CHAIN", None),
        ]);
        let profile = from_kind(NetworkKind::Mainnet);
        let banner = session_banner(&profile, Operation::Plan);
        assert!(banner.contains("active_network=mainnet"));
        assert!(banner.contains("operation=plan"));
        decide(&profile, Operation::Plan, false, None).expect_err("unauthorized plan");
        decide(&profile, Operation::Plan, true, Some("mainnet")).expect("plan");
        decide(&profile, Operation::Genesis, true, Some("mainnet")).expect("genesis");
        decide(&profile, Operation::OfficialName, true, Some("mainnet")).expect("claim");
        decide(&profile, Operation::Transition, true, Some("mainnet")).expect_err("transition");

        let keys = keys_for(profile.coin_type);
        let regtest_keys = keys_for(from_kind(NetworkKind::Regtest).coin_type);
        assert_ne!(keys.root.xonly, regtest_keys.root.xonly);
        let state = genesis_state_from(&keys, [0u8; 36]);
        let seal = seal_for_state(&state).expect("seal");
        let address = Address::from_script(
            ScriptBuf::from_bytes(seal.script_pubkey).as_script(),
            profile.bitcoin,
        )
        .expect("address")
        .to_string();
        assert!(address.starts_with("bc1p"), "{address}");
        let descriptor = crate::lineage::core_descriptor(&state);
        let signed = genesis_for(profile.network_byte, keys.root, &state);
        assert_eq!(signed.payload.get(2), Some(&profile.network_byte));
        verify(&signed).expect("genesis verifies");
        let entity = entity_id(&signed.payload);
        let sid = state_id(&entity, &encode_resulting_state(&state));
        let regtest = from_kind(NetworkKind::Regtest);
        let regtest_signed = genesis_for(
            regtest.network_byte,
            regtest_keys.root,
            &genesis_state_from(&regtest_keys, [0u8; 36]),
        );
        assert_ne!(entity, entity_id(&regtest_signed.payload));
        let name = "Unsafe Mainnet Vector Artist";
        let claim = official_name_claim(
            profile.network_byte,
            entity,
            sid,
            keys.controller_0,
            name,
            official_name_nonce(&entity, name),
        )
        .expect("claim");
        assert_eq!(claim.payload.get(2), Some(&profile.network_byte));
        verify(&claim).expect("claim verifies");
        let broadcast = refuse_broadcast(&profile).expect_err("broadcast");
        assert_eq!(broadcast, "mainnet broadcast is refused");
        let transport = match crate::chain::Node::connect_profile(&profile, false) {
            Err(err) => err,
            Ok(_) => panic!("network transport opened"),
        };
        assert_eq!(transport, "network transport is disabled");
        drop(_lock);

        let core = core_derive_address(&descriptor).expect("core");
        assert_eq!(core, address, "core {core} local {address}");
    }

    fn core_derive_address(descriptor: &str) -> Result<String, String> {
        let dir = std::env::temp_dir().join(format!(
            "o2a-mainnet-dry-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|err| err.to_string())?
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let _cleanup = Dir(dir.clone());
        std::fs::write(
            dir.join("bitcoin.conf"),
            "\
chain=main
server=1
listen=0
dnsseed=0
connect=0
maxconnections=0
disablewallet=1
discover=0
natpmp=0
printtoconsole=1
rpcbind=127.0.0.1
rpcallowip=127.0.0.1
",
        )
        .map_err(|err| err.to_string())?;
        let script = r#"
set -e
bitcoind -datadir=/data -conf=/data/bitcoin.conf >/data/bitcoind.log 2>&1 &
trap 'bitcoin-cli -datadir=/data stop >/dev/null 2>&1 || true' EXIT
for i in $(seq 1 80); do
  if [ -f /data/.cookie ]; then
    break
  fi
  sleep 0.25
done
test -f /data/.cookie
INFO=$(bitcoin-cli -datadir=/data getblockchaininfo)
printf '%s' "$INFO" | grep -q '"chain": "main"'
printf '%s' "$INFO" | grep -q '"blocks": 0,'
DESC=$(bitcoin-cli -datadir=/data getdescriptorinfo "$O2A_DESCRIPTOR")
CHECK=$(printf '%s' "$DESC" | sed -n 's/.*"descriptor": "\([^"]*\)".*/\1/p')
test -n "$CHECK"
RAW=$(bitcoin-cli -datadir=/data deriveaddresses "$CHECK")
ADDR=$(printf '%s' "$RAW" | sed -n 's/.*"\(bc1[^"]*\)".*/\1/p')
test -n "$ADDR"
printf 'address=%s\n' "$ADDR"
"#;
        let output = std::process::Command::new("docker")
            .env("DOCKER_CONTEXT", "default")
            .env("O2A_DESCRIPTOR", descriptor)
            .args([
                "run",
                "--rm",
                "--network",
                "none",
                "-e",
                "O2A_DESCRIPTOR",
                "-v",
                &format!("{}:/data", dir.display()),
                "--entrypoint",
                "sh",
                "bitcoin/bitcoin:31.1",
                "-c",
                script,
            ])
            .output()
            .map_err(|err| err.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(format!(
                "core derive failed: {}\n{stdout}\n{stderr}",
                output.status
            ));
        }
        stdout
            .lines()
            .find_map(|line| line.strip_prefix("address="))
            .map(str::to_string)
            .ok_or_else(|| format!("core address missing\n{stdout}\n{stderr}"))
    }

    struct Dir(std::path::PathBuf);

    impl Drop for Dir {
        fn drop(&mut self) {
            // Core's image writes the datadir as root. Wipe it inside that
            // image, with no network, before removing the host directory.
            let _ = std::process::Command::new("docker")
                .env("DOCKER_CONTEXT", "default")
                .args([
                    "run",
                    "--rm",
                    "--network",
                    "none",
                    "-v",
                    &format!("{}:/data", self.0.display()),
                    "--entrypoint",
                    "sh",
                    "bitcoin/bitcoin:31.1",
                    "-c",
                    "rm -rf /data/* /data/.[!.]* /data/..?*",
                ])
                .status();
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
