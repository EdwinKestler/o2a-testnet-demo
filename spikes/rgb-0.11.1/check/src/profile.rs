//! One network profile. Every other module reads this value and does not
//! keep a second network table.
//!
//! Genesis signs at `required_depth`: regtest 1, signet 1, mainnet 6.
//! Mainnet transport is an explicit read allowlist. Broadcast methods stay
//! refused.

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

    /// Plan, verify, genesis, and the one official-name claim.
    pub fn allowed_on_mainnet(self) -> bool {
        matches!(
            self,
            Operation::Plan | Operation::Verify | Operation::Genesis | Operation::OfficialName
        )
    }

    /// Plan, genesis, and the official_name claim use keys. Verify does not.
    pub fn needs_session_lock(self) -> bool {
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
        profile.create_miner_wallet = false;
        profile.offline = profile.rpc_url.is_none() || profile.electrum.is_none();
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
    if profile.kind == NetworkKind::Mainnet && operation.needs_session_lock() {
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
            "mainnet scope refuses {}; only plan, verify, genesis, and the official_name claim are allowed",
            operation.name()
        ));
    }
    if !operation.needs_session_lock() {
        return Ok(());
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
        if profile.kind == NetworkKind::Mainnet && operation.needs_session_lock() && authorize {
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
        "sendrawtransaction"
            | "sendtoaddress"
            | "sendmany"
            | "submitpackage"
            | "submitblock"
            | "blockchain.transaction.broadcast"
    ) || method.starts_with("send")
}

/// Headers, raw transactions, merkle proofs, unspent queries, chain info, and electrum reads.
pub fn is_read_method(method: &str) -> bool {
    matches!(
        method,
        "getblockchaininfo"
            | "getblockcount"
            | "getblockhash"
            | "getblockheader"
            | "getblock"
            | "getrawtransaction"
            | "gettxout"
            | "gettxoutproof"
            | "verifytxoutproof"
            | "scantxoutset"
            | "listunspent"
            | "blockchain.headers.subscribe"
            | "blockchain.block.header"
            | "blockchain.block.headers"
            | "blockchain.transaction.get"
            | "blockchain.transaction.get_merkle"
            | "blockchain.transaction.id_from_pos"
            | "server.ping"
            | "server.version"
            | "server.features"
    )
}

/// Runs before any socket. On mainnet, broadcast is refused and every other method must be on the read list.
pub fn mainnet_method_gate(mainnet: bool, method: &str) -> Result<(), String> {
    if !mainnet {
        return Ok(());
    }
    if is_broadcast_method(method) {
        return Err("mainnet broadcast is refused".into());
    }
    if !is_read_method(method) {
        return Err(format!("mainnet transport is read-only; refused {method}"));
    }
    Ok(())
}

pub struct CliArgs {
    pub command: Option<String>,
    pub authorize: bool,
    pub seal: Option<String>,
    pub json: bool,
    pub dir: Option<String>,
}

pub fn split_args(args: &[String]) -> Result<CliArgs, String> {
    let mut authorize = false;
    let mut command = None;
    let mut seal = None;
    let mut expect_seal = false;
    let mut json = false;
    let mut dir = None;
    for arg in args {
        if expect_seal {
            if arg.starts_with('-') || arg.is_empty() {
                return Err("genesis --seal requires an outpoint".into());
            }
            seal = Some(arg.clone());
            expect_seal = false;
            continue;
        }
        if arg == "--authorize-mainnet" {
            authorize = true;
        } else if arg == "--json" {
            json = true;
        } else if arg == "--seal" {
            expect_seal = true;
        } else if command.is_none() && !arg.starts_with('-') {
            command = Some(arg.clone());
        } else if command.as_deref() == Some("publish-package")
            && dir.is_none()
            && !arg.starts_with('-')
        {
            dir = Some(arg.clone());
        } else {
            return Err(format!("unknown argument {arg}"));
        }
    }
    if expect_seal {
        return Err("genesis --seal requires an outpoint".into());
    }
    if json && !matches!(command.as_deref(), Some("verify") | Some("signet-verify")) {
        return Err("--json is only valid with verify".into());
    }
    if command.as_deref() == Some("publish-package") && dir.is_none() {
        return Err("publish-package requires a destination directory".into());
    }
    Ok(CliArgs {
        command,
        authorize,
        seal,
        json,
        dir,
    })
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
    use o2a_demo_core::{
        encode_resulting_state, entity_id, genesis_for, genesis_state_from, keys_for,
        official_name_claim, official_name_nonce, seal_for_state, state_id, verify,
    };
    use rgbstd::bitcoin::{Address, ScriptBuf};

    use super::*;

    fn lock_env(pairs: &[(&str, Option<&str>)]) -> crate::test_env::EnvLock {
        crate::test_env::lock_env(pairs)
    }

    fn must_err<T>(result: Result<T, String>) -> String {
        match result {
            Err(err) => err,
            Ok(_) => panic!("expected an error"),
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
        assert_eq!(mainnet.required_depth, 6);
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
        assert_eq!(signet.required_depth, 1);
        decide(&signet, Operation::Genesis, false, None).expect("genesis follows the profile");
        decide(&signet, Operation::OfficialName, false, None).expect("claim follows the profile");

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
        decide(&verify, Operation::Verify, false, None).expect("verify follows the profile");
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
        let plan = session_banner(&from_kind(NetworkKind::Mainnet), Operation::Plan);
        let stage_verify = session_banner(&from_kind(NetworkKind::Mainnet), Operation::Verify);
        assert!(plan.contains("operation=plan"));
        assert!(plan.contains("type mainnet to confirm this plan session"));
        assert!(!stage_verify.contains("type mainnet"));
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
        let plan = decide(&mainnet, Operation::Plan, false, None).expect_err("plan flag");
        assert!(plan.contains("--authorize-mainnet"), "{plan}");
        decide(&mainnet, Operation::Plan, true, Some("mainnet")).expect("authorized plan");
        decide(&mainnet, Operation::Verify, false, None).expect("verify");
        decide(&mainnet, Operation::OfficialName, true, Some("mainnet")).expect("claim");
        let claim = decide(&mainnet, Operation::OfficialName, false, None).expect_err("claim flag");
        assert!(claim.contains("--authorize-mainnet"), "{claim}");
        let plan_cli = begin_cli(&mainnet, Operation::Plan, false).expect_err("plan stdin");
        assert!(plan_cli.contains("--authorize-mainnet"), "{plan_cli}");
        begin_cli(&mainnet, Operation::Verify, true).expect("verify does not read stdin");
        let genesis = begin_cli(&mainnet, Operation::Genesis, false).expect_err("genesis");
        assert!(genesis.contains("--authorize-mainnet"), "{genesis}");
        let transition_cli =
            begin_cli(&mainnet, Operation::Transition, true).expect_err("transition stdin");
        assert!(transition_cli.contains("scope"), "{transition_cli}");
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
    fn genesis_accepts_a_seal_outpoint() {
        let parsed = split_args(&[
            "--authorize-mainnet".into(),
            "genesis".into(),
            "--seal".into(),
            "aa:1".into(),
        ])
        .expect("parse");
        assert_eq!(parsed.command.as_deref(), Some("genesis"));
        assert!(parsed.authorize);
        assert_eq!(parsed.seal.as_deref(), Some("aa:1"));
        let missing = must_err(split_args(&["genesis".into(), "--seal".into()]));
        assert!(missing.contains("outpoint"), "{missing}");
        let unknown = must_err(split_args(&["plan".into(), "extra".into()]));
        assert!(unknown.contains("unknown argument"), "{unknown}");
        let json = split_args(&["verify".into(), "--json".into()]).expect("json");
        assert!(json.json);
        assert!(json.dir.is_none());
        let alias = split_args(&["--json".into(), "signet-verify".into()]).expect("alias");
        assert!(alias.json);
        let rejected = must_err(split_args(&["plan".into(), "--json".into()]));
        assert!(
            rejected.contains("--json is only valid with verify"),
            "{rejected}"
        );
        let missing = must_err(split_args(&["publish-package".into()]));
        assert!(missing.contains("destination directory"), "{missing}");
        let package =
            split_args(&["publish-package".into(), "/tmp/o2a-package".into()]).expect("package");
        assert_eq!(package.dir.as_deref(), Some("/tmp/o2a-package"));
        assert!(!package.json);
        let extra = must_err(split_args(&["verify".into(), "/tmp/o2a-package".into()]));
        assert!(extra.contains("unknown argument"), "{extra}");
    }

    #[test]
    fn mainnet_read_only_transport_refuses_broadcast_without_a_socket() {
        let _lock = lock_env(&[
            ("O2A_NETWORK", Some("mainnet")),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", None),
            ("BITCOIN_RPC", Some("http://127.0.0.1:1")),
            ("ELECTRUM", Some("127.0.0.1:1")),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
            ("BITCOIN_RPC_USER", Some("reader")),
            ("BITCOIN_RPC_PASSWORD", Some("reader-pass")),
        ]);
        let profile = load().expect("mainnet");
        assert!(!profile.offline);
        assert!(!profile.create_miner_wallet);
        allow_transport(&profile).expect("configured read endpoints");
        mainnet_method_gate(true, "getrawtransaction").expect("raw transaction");
        mainnet_method_gate(true, "getblockheader").expect("header");
        mainnet_method_gate(true, "gettxoutproof").expect("merkle proof");
        mainnet_method_gate(true, "gettxout").expect("unspent");
        mainnet_method_gate(true, "listunspent").expect("listunspent");
        mainnet_method_gate(true, "getblockchaininfo").expect("chain info");
        mainnet_method_gate(true, "blockchain.transaction.get").expect("electrum read");
        mainnet_method_gate(true, "blockchain.headers.subscribe").expect("electrum headers");
        let broadcast = mainnet_method_gate(true, "sendrawtransaction").expect_err("broadcast");
        assert_eq!(broadcast, "mainnet broadcast is refused");
        let electrum_broadcast =
            mainnet_method_gate(true, "blockchain.transaction.broadcast").expect_err("electrum");
        assert_eq!(electrum_broadcast, "mainnet broadcast is refused");
        let closed = mainnet_method_gate(true, "generatetoaddress").expect_err("mine");
        assert!(closed.contains("read-only"), "{closed}");
        let fund = mainnet_method_gate(true, "fundrawtransaction").expect_err("fund");
        assert!(fund.contains("read-only"), "{fund}");
        let scope =
            decide(&profile, Operation::Broadcast, true, Some("mainnet")).expect_err("scope");
        assert!(scope.contains("scope"), "{scope}");
        let node = match crate::chain::Node::connect_profile(&profile, false) {
            Ok(node) => node,
            Err(err) => panic!("read endpoints were refused before a socket: {err}"),
        };
        let sent = node
            .call(false, "sendrawtransaction", serde_json::json!(["00"]))
            .expect_err("broadcast");
        assert_eq!(sent, "mainnet broadcast is refused");
        assert!(!sent.contains("rpc connect"), "{sent}");
        let mined = node
            .call(false, "generatetoaddress", serde_json::json!([1, "bc1q"]))
            .expect_err("mine");
        assert!(mined.contains("read-only"), "{mined}");
        assert!(!mined.contains("rpc connect"), "{mined}");
        let mut resolver = crate::chain::ElectrumResolver::open().expect("resolver");
        let relay = resolver
            .call("blockchain.transaction.broadcast", serde_json::json!([]))
            .expect_err("electrum broadcast");
        assert_eq!(relay, "mainnet broadcast is refused");
        assert!(!relay.contains("electrum:"), "{relay}");

        drop(_lock);
        let _missing = lock_env(&[
            ("O2A_NETWORK", Some("mainnet")),
            ("O2A_DEMO_NETWORK", None),
            ("RGB_CHAIN", None),
            ("BITCOIN_RPC", None),
            ("ELECTRUM", None),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
            ("BITCOIN_RPC_USER", None),
            ("BITCOIN_RPC_PASSWORD", None),
        ]);
        let bare = load().expect("bare mainnet");
        assert!(bare.offline);
        let err = allow_transport(&bare).expect_err("no endpoints");
        assert_eq!(err, "backend endpoints are not configured");
        let closed_node = match crate::chain::Node::connect_profile(&bare, false) {
            Err(err) => err,
            Ok(_) => panic!("unconfigured mainnet opened"),
        };
        assert_eq!(closed_node, "backend endpoints are not configured");
        assert!(!closed_node.contains("rpc connect"));
    }

    #[test]
    fn offline_mainnet_dry_run_signs_against_canned_funding() {
        let _lock = lock_env(&[
            ("O2A_DEMO_SEED_FILE", None),
            ("O2A_DEMO_ENTITY", None),
            ("O2A_DEMO_DELAY", None),
            ("O2A_DEMO_THRESHOLD", None),
            ("O2A_DEMO_NETWORK", None),
            ("O2A_NETWORK", Some("mainnet")),
            ("RGB_CHAIN", None),
            ("BITCOIN_RPC", None),
            ("ELECTRUM", None),
            ("BITCOIN_WALLET", None),
            ("BITCOIN_COOKIE", None),
            ("BITCOIN_RPC_USER", None),
            ("BITCOIN_RPC_PASSWORD", None),
        ]);
        let profile = load().expect("mainnet");
        assert!(profile.offline);
        assert_eq!(profile.network_byte, 0);
        assert_eq!(profile.coin_type, 0);
        assert_eq!(profile.required_depth, 6);
        let banner = session_banner(&profile, Operation::Plan);
        assert!(banner.contains("active_network=mainnet"));
        assert!(banner.contains("operation=plan"));
        assert!(banner.contains("type mainnet to confirm this plan session"));
        let missing_plan = decide(&profile, Operation::Plan, false, None).expect_err("plan flag");
        assert!(
            missing_plan.contains("--authorize-mainnet"),
            "{missing_plan}"
        );
        decide(&profile, Operation::Plan, true, Some("mainnet")).expect("authorized plan");
        decide(&profile, Operation::Verify, false, None).expect("verify");
        decide(&profile, Operation::Genesis, true, Some("mainnet")).expect("authorized genesis");
        decide(&profile, Operation::OfficialName, true, Some("mainnet")).expect("claim");
        let transition =
            decide(&profile, Operation::Transition, true, Some("mainnet")).expect_err("transition");
        assert!(transition.contains("scope"), "{transition}");
        let refused_plan = crate::lineage::plan(false).expect_err("plan command");
        assert!(
            refused_plan.contains("--authorize-mainnet"),
            "{refused_plan}"
        );

        // Test vector only. The CLI mainnet plan command does not call this path.
        let keys = keys_for(profile.coin_type);
        let regtest_keys = keys_for(from_kind(NetworkKind::Regtest).coin_type);
        assert_ne!(keys.root.xonly, regtest_keys.root.xonly);
        let planned = genesis_state_from(&keys, [0u8; 36]);
        let seal = seal_for_state(&planned).expect("seal");
        let address = Address::from_script(
            ScriptBuf::from_bytes(seal.script_pubkey.clone()).as_script(),
            profile.bitcoin,
        )
        .expect("address")
        .to_string();
        assert!(address.starts_with("bc1p"), "{address}");
        let policy = crate::lineage::core_descriptor(&planned);
        assert!(policy.starts_with("tr("), "{policy}");

        let (outpoint, view) = crate::lineage::canned_funding(
            &seal.script_pubkey,
            4_294_967_294,
            6,
            true,
            b"canned-mainnet-proof",
            true,
        );
        let draft = crate::lineage::draft_genesis(&profile, &outpoint, &view).expect("genesis");
        assert_eq!(draft.signed.payload.get(2), Some(&0u8));
        verify(&draft.signed).expect("genesis verifies");
        assert_eq!(draft.signed.payload.get(5..37), Some(&[0u8; 32][..]));
        let entity = entity_id(&draft.signed.payload);
        let regtest = from_kind(NetworkKind::Regtest);
        let regtest_signed = genesis_for(
            regtest.network_byte,
            regtest_keys.root,
            &genesis_state_from(&regtest_keys, [0u8; 36]),
        );
        assert_ne!(entity, entity_id(&regtest_signed.payload));
        let sid = state_id(&entity, &encode_resulting_state(&draft.state));
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
        assert_eq!(claim.payload.get(2), Some(&0u8));
        verify(&claim).expect("claim verifies");

        let shallow = crate::lineage::canned_funding(
            &seal.script_pubkey,
            4_294_967_294,
            5,
            true,
            b"canned-mainnet-proof",
            true,
        );
        let err = must_err(crate::lineage::draft_genesis(
            &profile, &shallow.0, &shallow.1,
        ));
        assert!(err.contains("stays unsigned"), "{err}");
        let replaceable = crate::lineage::canned_funding(
            &seal.script_pubkey,
            4_294_967_293,
            6,
            true,
            b"canned-mainnet-proof",
            true,
        );
        let err = must_err(crate::lineage::draft_genesis(
            &profile,
            &replaceable.0,
            &replaceable.1,
        ));
        assert!(err.contains("replaceable"), "{err}");
        let spent = crate::lineage::canned_funding(
            &seal.script_pubkey,
            4_294_967_295,
            6,
            false,
            b"canned-mainnet-proof",
            true,
        );
        let err = must_err(crate::lineage::draft_genesis(&profile, &spent.0, &spent.1));
        assert!(err.contains("spent"), "{err}");
        let wrong = crate::lineage::canned_funding(
            &[0x51],
            4_294_967_294,
            6,
            true,
            b"canned-mainnet-proof",
            true,
        );
        let err = must_err(crate::lineage::draft_genesis(&profile, &wrong.0, &wrong.1));
        assert!(err.contains("planned seal policy"), "{err}");
        let unproved =
            crate::lineage::canned_funding(&seal.script_pubkey, 4_294_967_294, 6, true, b"", true);
        let err = must_err(crate::lineage::draft_genesis(
            &profile,
            &unproved.0,
            &unproved.1,
        ));
        assert!(err.contains("merkle"), "{err}");
        let bad_header = crate::lineage::canned_funding(
            &seal.script_pubkey,
            4_294_967_294,
            6,
            true,
            b"canned-mainnet-proof",
            false,
        );
        let err = must_err(crate::lineage::draft_genesis(
            &profile,
            &bad_header.0,
            &bad_header.1,
        ));
        assert!(err.contains("block header"), "{err}");

        let refused = crate::lineage::genesis(false, Some(outpoint.as_str())).expect_err("lock");
        assert!(refused.contains("--authorize-mainnet"), "{refused}");
        assert!(!refused.contains("rpc connect"), "{refused}");
        let broadcast = refuse_broadcast(&profile).expect_err("broadcast");
        assert_eq!(broadcast, "mainnet broadcast is refused");
    }
}
