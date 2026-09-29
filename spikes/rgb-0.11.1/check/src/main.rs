//! Regtest compatibility checks for RGB 0.11.1. No wallet claims the seal UTXOs.

mod chain;
mod format;
mod lineage;
mod profile;
mod schema;
mod spend;

#[cfg(test)]
mod test_env;

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use amplify::confinement::Confined;
use o2a_demo_core::{
    demo_genesis_state, demo_keys, demo_rotation_state, entity_id, genesis_with, seal_for_state,
    state_id,
};
use rgbstd::bitcoin::consensus::encode::deserialize;
use rgbstd::bitcoin::hashes::Hash;
use rgbstd::bitcoin::{Address, Amount, OutPoint, ScriptBuf, Transaction, Txid};
use rgbstd::containers::{Consignment, ConsignmentExt, Fascia};
use rgbstd::contract::AllocatedState;
use rgbstd::persistence::{ContractStateRead, Stock};
use rgbstd::validation::{Status, ValidationConfig, WitnessOrdProvider};
use rgbstd::{Operation, Opout, OutputSeal, SecretSeal};
use schemata::NIA_SCHEMA_ID;
use serde_json::{json, Value};
use strict_encoding::{StrictDeserialize, StrictSerialize};
use strict_types::TypeSystem;

const MAX_CONSIGNMENT: usize = 4_000_000;
const BLINDING: u64 = 0x0A11_0110;
const MINER_FEE: u64 = 15_000;
const NEXT_SATS: u64 = 100_000;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match profile::split_args(&args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("setup failed: {err}");
            std::process::exit(1);
        }
    };
    let authorize = parsed.authorize;
    let result = match parsed.command.as_deref() {
        Some("lineage") => lineage::run(authorize),
        Some("plan") => lineage::plan(authorize),
        Some("preflight") => lineage::preflight(authorize, parsed.seal.as_deref(), parsed.json),
        Some("genesis") => lineage::genesis(authorize, parsed.seal.as_deref()),
        Some("claim") => lineage::claim(authorize),
        Some("verify") => lineage::stage_verify(authorize, parsed.json),
        Some("signet-genesis") => {
            eprintln!(
                "note: signet-genesis is deprecated; use genesis --seal <outpoint>. This alias sets no network."
            );
            lineage::genesis(authorize, parsed.seal.as_deref())
        }
        Some("signet-claim") => {
            eprintln!("note: signet-claim is deprecated; use claim. This alias sets no network.");
            lineage::claim(authorize)
        }
        Some("signet-verify") => {
            eprintln!("note: signet-verify is deprecated; use verify. This alias sets no network.");
            lineage::stage_verify(authorize, parsed.json)
        }
        Some("publish-package") => (|| -> Result<(), String> {
            // Offline and keyless. The profile supplies the network label only.
            let dest = parsed
                .dir
                .clone()
                .ok_or("publish-package requires a destination directory")?;
            let active = profile::load()?;
            let source = PathBuf::from(
                std::env::var("RGB011_EVIDENCE").map_err(|_| "RGB011_EVIDENCE is required")?,
            );
            format::publish_package(
                &source,
                std::path::Path::new(&dest),
                active.kind.label(),
                active.network_byte,
            )
        })(),
        _ => run(authorize),
    };
    if let Err(err) = result {
        eprintln!("setup failed: {err}");
        std::process::exit(1);
    }
}

struct Log {
    file: File,
    dir: PathBuf,
}

impl Log {
    fn open(dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let file = File::create(dir.join("cases.txt")).map_err(|err| err.to_string())?;
        Ok(Self { file, dir })
    }

    fn line(&mut self, text: &str) {
        println!("{text}");
        let _ = std::io::stdout().flush();
        let _ = writeln!(self.file, "{text}");
        let _ = self.file.flush();
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        fs::write(self.dir.join(name), bytes).map_err(|err| err.to_string())
    }

    fn case(&mut self, id: &str, expected: &str, observed: &str, pass: bool) {
        self.line(&format!("CASE {id}"));
        self.line(&format!("expected: {expected}"));
        self.line(&format!("observed: {observed}"));
        self.line(&format!("result: {}", if pass { "PASS" } else { "FAIL" }));
    }
}

struct TentativeOrd;

impl WitnessOrdProvider for TentativeOrd {
    fn witness_ord(
        &self,
        _witness_id: Txid,
    ) -> Result<rgbstd::vm::WitnessOrd, rgbstd::validation::WitnessResolverError> {
        Ok(rgbstd::vm::WitnessOrd::Tentative)
    }
}

fn run(authorize: bool) -> Result<(), String> {
    let active = profile::load()?;
    profile::begin_cli(&active, profile::Operation::Compatibility, authorize)?;
    profile::require_regtest_command(&active)?;
    let dir =
        PathBuf::from(std::env::var("RGB011_EVIDENCE").map_err(|_| "RGB011_EVIDENCE is required")?);
    let mut log = Log::open(dir)?;
    let _ = rgb::resolvers::ContractIssueResolver;
    if std::env::var("O2A_DEMO_SEED_FILE").is_err() {
        return Err("O2A_DEMO_SEED_FILE is required".into());
    }
    let node = chain::Node::connect()?;
    let height = node.height()?;
    let h0 = match height {
        0 => node.mine(101)?,
        101 => {
            chain::wait_electrs(101)?;
            101
        }
        other => {
            return Err(format!(
                "chain height is {other}; reset the o2a-rgb011 volume so H0 is recorded on a fresh regtest"
            ));
        }
    };
    log.line(&format!("H0 {h0}"));
    log.line(&format!("network {}", active.kind.label()));
    log.line(&format!(
        "rgb-api link ContractIssueResolver is present; RgbWallet and rgb-lib are not called"
    ));

    let prepared =
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(schema::identity_schema)) {
            Ok(schema) => schema,
            Err(_) => {
                log.case(
                    "C1",
                    "O2A identity schema builds and a genesis validates",
                    "identity_schema panicked while resolving O2aDigest",
                    false,
                );
                return Ok(());
            }
        };
    let schema_id = prepared.schema.schema_id();
    let validators_none = prepared.schema.genesis.validator.is_none()
        && prepared
            .schema
            .transitions
            .values()
            .all(|details| details.transition_schema.validator.is_none());
    log.line(&format!("schema_id {schema_id}"));
    log.line(&format!("nia_schema_id {NIA_SCHEMA_ID}"));
    log.line(&format!("validators_none {validators_none}"));
    if schema_id == NIA_SCHEMA_ID {
        log.case("C1", "custom schema id", "schema id equals NIA", false);
        return Ok(());
    }

    let keys = demo_keys();
    let fee = spend::local_key()?;
    log.line(&format!("fee_xonly {}", hex::encode(fee.xonly)));
    log.line(&format!("root_xonly {}", hex::encode(keys.root.xonly)));
    let placeholder = demo_genesis_state([0u8; 36]);
    let genesis_script = seal_for_state(&placeholder).map_err(|err| err.to_string())?;
    let rotation_state = demo_rotation_state(1, [0u8; 32], [0u8; 36], [0u8; 36]);
    let rotation_script = seal_for_state(&rotation_state).map_err(|err| err.to_string())?;
    let real_placeholder = demo_genesis_state([1u8; 36]);
    let real_script = seal_for_state(&real_placeholder).map_err(|err| err.to_string())?;
    let scripts_ignore_outpoint = genesis_script.script_pubkey == real_script.script_pubkey
        && genesis_script.scripts == real_script.scripts;
    log.line(&format!(
        "o2a_script_ignores_outpoint {scripts_ignore_outpoint}"
    ));

    let seal_script = ScriptBuf::from_bytes(genesis_script.script_pubkey.clone());
    let next_script = ScriptBuf::from_bytes(rotation_script.script_pubkey.clone());
    let outputs = [
        ("A", seal_script.clone(), 100_000u64),
        ("R", seal_script.clone(), 110_000),
        ("C", seal_script.clone(), 120_000),
        ("D", seal_script, 130_000),
        ("fee4", fee.script.clone(), 60_000),
        ("fee5", fee.script.clone(), 61_000),
        ("fee7", fee.script.clone(), 62_000),
    ];
    let funded = fund(&node, &outputs)?;
    let f_height = funded.height;
    log.line(&format!("F {f_height}"));
    log.line("funding_split one output per transaction; Core rejects a duplicated address");
    for (label, point) in &funded.outs {
        log.line(&format!("vout {label} {} {}", point.txid, point.vout));
    }

    let mut stock = Stock::in_memory();
    let seal_a = issued_contract(
        &mut log,
        &mut stock,
        &prepared,
        &keys,
        &genesis_script,
        funded.outs["A"].txid,
        funded.outs["A"].vout,
        100_000,
        schema::TS_ISSUE,
        "A",
    )?;
    log.case(
        "C1",
        "schema with declarative identity, 32-byte digest, rotate/recover/revoke, validator None, genesis validates",
        &format!(
            "schema {schema_id} nia {NIA_SCHEMA_ID} validators_none {validators_none} contract {} digest {} report {}",
            seal_a.contract_id, seal_a.digest_hex, one_line(&seal_a.report)
        ),
        validators_none && scripts_ignore_outpoint && seal_a.valid && schema_id != NIA_SCHEMA_ID,
    );
    let script_match = seal_a.script_match;
    log.case(
        "C2",
        "identity right on the external O2A seal outpoint validates; script match is outside RGB",
        &format!(
            "outpoint {} o2a_script_match {script_match} rgb_seal_has_no_script_field true assignments {} report {}",
            seal_a.outpoint, one_line(&seal_a.assignments), one_line(&seal_a.report)
        ),
        seal_a.valid && script_match && !seal_a.assignments.contains("no assignment"),
    );

    let seal_r = issued_contract(
        &mut log,
        &mut stock,
        &prepared,
        &keys,
        &genesis_script,
        funded.outs["R"].txid,
        funded.outs["R"].vout,
        110_000,
        schema::TS_ISSUE,
        "R",
    )?;
    let seal_c = issued_contract(
        &mut log,
        &mut stock,
        &prepared,
        &keys,
        &genesis_script,
        funded.outs["C"].txid,
        funded.outs["C"].vout,
        120_000,
        schema::TS_ISSUE,
        "C",
    )?;
    let seal_d1 = issued_contract(
        &mut log,
        &mut stock,
        &prepared,
        &keys,
        &genesis_script,
        funded.outs["D"].txid,
        funded.outs["D"].vout,
        130_000,
        1_759_017_700,
        "D1",
    )?;
    let seal_d2 = issued_contract(
        &mut log,
        &mut stock,
        &prepared,
        &keys,
        &genesis_script,
        funded.outs["D"].txid,
        funded.outs["D"].vout,
        130_000,
        1_759_017_701,
        "D2",
    )?;
    let d_same_digest = seal_d1.digest_hex == seal_d2.digest_hex;
    let d_distinct = seal_d1.contract_id != seal_d2.contract_id;
    log.line(&format!(
        "C7_unspent same_digest {d_same_digest} distinct_ids {d_distinct} d1 {} d2 {}",
        one_line(&seal_d1.report),
        one_line(&seal_d2.report)
    ));

    let revoke = schema::revoke_transition(
        &prepared,
        seal_a.contract_id,
        seal_a.opout,
        AllocatedState::Void,
        13,
    );
    match &revoke {
        Ok(transition) => log.line(&format!(
            "revoke_offline opid {} not_broadcast",
            transition.id()
        )),
        Err(err) => log.line(&format!("revoke_offline error {err}")),
    }

    let rotate = schema::right_transition(
        &prepared,
        seal_a.contract_id,
        "rotate",
        seal_a.opout,
        AllocatedState::Void,
        1,
        BLINDING,
        11,
    )?;
    let c9_tree = spend::tapret_probe(rotate.clone(), true);
    let c9_key = spend::tapret_probe(rotate.clone(), false);
    log.line("CASE C9");
    log.line("expected: report Tapret on a script-tree host and a key-only host; do not broadcast");
    log.line(&format!("observed: tree {c9_tree}; key-only {c9_key}"));
    log.line("result: REPORT");

    let fee_only = stock
        .contract_assignments_for(seal_a.contract_id, [funded.outs["fee4"]])
        .map(|map| map.is_empty())
        .unwrap_or(false);
    let wallet_note = format!(
        "pay.rs:112-121 ContractOutpointsFilter::should_include requires wallet.filter_unspent().should_include and a non-empty stock.contract_assignments_for. pay.rs:535-537 returns CompositionError::InsufficientState when that selection is empty. pay.rs:554-570 calls set_rgb_close_method, set_as_unmodifiable, and rgb_embed; those psrgbt methods do not check wallet ownership. wallet.rs:49-94 RgbWallet wraps WalletProvider. This binary does not construct RgbWallet and does not enable the bp feature. stock finds the seal ({}). A fee-only unspent set does not ({fee_only}). The spike puts the external seal on the PSBT itself.",
        !seal_a.assignments.contains("no assignment")
    );
    log.line(&format!("C3 {wallet_note}"));

    let spent = spend::commit_opret(
        seal_a.outpoint,
        Amount::from_sat(100_000),
        &genesis_script,
        spend::LeafSpend::Controller {
            key: keys.seal_controller_0,
        },
        funded.outs["fee4"],
        Amount::from_sat(60_000),
        &fee.keypair,
        next_script.clone(),
        Amount::from_sat(NEXT_SATS),
        fee.script.clone(),
        Amount::from_sat(45_000),
        u32::MAX,
        rotate,
    );
    let spent = match spent {
        Ok(spent) => spent,
        Err(err) => {
            log.case(
                "C3",
                "PSBT embeds and commits the rotate transition",
                &err,
                false,
            );
            log.case("C4", "broadcast, mine, two validators agree", &err, false);
            finish_rest(
                &mut log,
                &node,
                &mut stock,
                &prepared,
                &genesis_script,
                &fee,
                &keys,
                &funded,
                &seal_r,
                &seal_c,
                &seal_d1,
                &seal_d2,
            )?;
            return Ok(());
        }
    };
    log.write("c4-rotate.hex", spent.hex.as_bytes())?;
    let c3_ok = spent.fascia.witness_id() == spent.txid;
    log.case(
        "C3",
        "generic PSBT commits the transition; wallet ownership is not assumed by psrgbt",
        &format!(
            "witness {} txid {} transition {} {wallet_note}",
            spent.fascia.witness_id(),
            spent.txid,
            spent.transition_id
        ),
        c3_ok,
    );
    let c4 = broadcast_commit(&mut log, &node, &mut stock, &prepared.types, &spent, "c4")?;
    log.case(
        "C4",
        "script-path broadcast and two independent electrs validators return the same bytes",
        &c4.text,
        c3_ok && c4.pass,
    );

    let recover = schema::right_transition(
        &prepared,
        seal_r.contract_id,
        "recover",
        seal_r.opout,
        AllocatedState::Void,
        1,
        BLINDING,
        12,
    )?;
    let recovery = spend::commit_opret(
        seal_r.outpoint,
        Amount::from_sat(110_000),
        &genesis_script,
        spend::LeafSpend::Recovery {
            keys: [
                keys.seal_recovery_0,
                keys.seal_recovery_1,
                keys.seal_recovery_2,
            ],
        },
        funded.outs["fee5"],
        Amount::from_sat(61_000),
        &fee.keypair,
        ScriptBuf::from_bytes(genesis_script.script_pubkey.clone()),
        Amount::from_sat(NEXT_SATS),
        fee.script.clone(),
        Amount::from_sat(46_000),
        10,
        recover,
    );
    match recovery {
        Ok(recovery) => {
            log.write("c5-recover.hex", recovery.hex.as_bytes())?;
            let gate = relative_lock(&node, &recovery.hex)?;
            let pass = gate.contains("non-BIP68-final")
                && gate.contains("allowed_at ")
                && !gate.contains("FAIL")
                && gate.contains("mined ");
            let accepted_early = gate.contains("FAIL accepted");
            log.case(
                "C5",
                "same recovery hex is non-BIP68-final before maturity and mines after it",
                &gate,
                pass && !accepted_early,
            );
            if pass {
                let _ = consume_and_validate(
                    &mut log,
                    &node,
                    &mut stock,
                    &prepared.types,
                    &recovery,
                    "c5",
                );
            }
        }
        Err(err) => log.case("C5", "recovery leaf spend", &err, false),
    }

    finish_rest(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        &genesis_script,
        &fee,
        &keys,
        &funded,
        &seal_r,
        &seal_c,
        &seal_d1,
        &seal_d2,
    )?;
    log.line("checks_finished");
    Ok(())
}

struct Issued {
    contract_id: rgbstd::ContractId,
    opout: Opout,
    outpoint: OutPoint,
    digest_hex: String,
    report: String,
    valid: bool,
    script_match: bool,
    assignments: String,
    bytes: Vec<u8>,
}

fn issued_contract(
    log: &mut Log,
    stock: &mut Stock,
    prepared: &schema::IdentitySchema,
    keys: &o2a_demo_core::DemoKeys,
    expected_seal: &o2a_demo_core::SealScript,
    txid: Txid,
    vout: u32,
    _sats: u64,
    timestamp: i64,
    label: &str,
) -> Result<Issued, String> {
    let outpoint = OutPoint { txid, vout };
    let canonical = canonical_outpoint(txid, vout);
    let state = demo_genesis_state(canonical);
    let seal = seal_for_state(&state).map_err(|err| err.to_string())?;
    let script_match = seal.script_pubkey == expected_seal.script_pubkey
        && seal.scripts == expected_seal.scripts
        && seal.merkle_root == expected_seal.merkle_root;
    let signed = genesis_with(keys.root, &state);
    let entity = entity_id(&signed.payload);
    let digest = signed.digest;
    log.line(&format!(
        "{label} entity {} digest {} state {} outpoint {txid}:{vout}",
        hex::encode(entity),
        hex::encode(digest),
        hex::encode(state_id(
            &entity,
            &o2a_demo_core::encode_resulting_state(&state)
        ))
    ));
    let issued = schema::issue_at(prepared, digest, outpoint, BLINDING, timestamp)?;
    let contract_id = ConsignmentExt::contract_id(&*issued);
    let opout = Opout::new(
        ConsignmentExt::genesis(&*issued).id(),
        schema::OS_IDENTITY,
        0,
    );
    let bytes = consignment_bytes(&*issued)?;
    log.write(&format!("{label}.strict"), &bytes)?;
    let report = dual_validate::<false>(&bytes, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    let valid = report.starts_with("Consignment is valid");
    let imported = stock.import_contract(
        issued.into_valid_contract(),
        chain::ElectrumResolver::open()?,
    );
    if let Err(err) = &imported {
        log.line(&format!("{label} import {err}"));
    }
    let assignments = rights_text(stock, contract_id, outpoint);
    log.line(&format!("{label} report {}", one_line(&report)));
    log.line(&format!("{label} assignments {}", one_line(&assignments)));
    Ok(Issued {
        contract_id,
        opout,
        outpoint,
        digest_hex: hex::encode(digest),
        report,
        valid: valid && imported.is_ok(),
        script_match,
        assignments,
        bytes,
    })
}

struct Funded {
    height: u32,
    outs: std::collections::BTreeMap<&'static str, OutPoint>,
}

fn fund(node: &chain::Node, outputs: &[(&'static str, ScriptBuf, u64)]) -> Result<Funded, String> {
    while spendable_count(node)? < outputs.len() {
        node.mine(1)?;
    }
    let mut outs = std::collections::BTreeMap::new();
    for (label, script, sats) in outputs {
        outs.insert(*label, fund_one(node, script, *sats)?);
    }
    let height = node.mine(1)?;
    Ok(Funded { height, outs })
}

fn spendable_count(node: &chain::Node) -> Result<usize, String> {
    let unspent = node.call(true, "listunspent", json!([1]))?;
    Ok(unspent.as_array().map(|items| items.len()).unwrap_or(0))
}

fn fund_one(node: &chain::Node, script: &ScriptBuf, sats: u64) -> Result<OutPoint, String> {
    let unspent = node.call(true, "listunspent", json!([1]))?;
    let utxo = unspent
        .as_array()
        .and_then(|items| items.first())
        .ok_or("no mature wallet output")?
        .clone();
    let inputs =
        json!([{ "txid": utxo["txid"], "vout": utxo["vout"], "sequence": 4_294_967_294u32 }]);
    let address =
        Address::from_script(script, profile::load()?.bitcoin).map_err(|err| err.to_string())?;
    let raw = node
        .call(
            true,
            "createrawtransaction",
            json!([inputs, [{ address.to_string(): btc(sats) }], 0, false]),
        )?
        .as_str()
        .ok_or("createrawtransaction")?
        .to_string();
    let funded = match node.call(
        true,
        "fundrawtransaction",
        json!([raw, { "changePosition": 1, "fee_rate": 5, "replaceable": false }]),
    ) {
        Ok(value) => value,
        Err(_) => node.call(
            true,
            "fundrawtransaction",
            json!([raw, { "changePosition": 1, "replaceable": false }]),
        )?,
    };
    let funded_hex = funded
        .get("hex")
        .and_then(Value::as_str)
        .ok_or("fund hex")?;
    let signed = node.call(true, "signrawtransactionwithwallet", json!([funded_hex]))?;
    if signed.get("complete").and_then(Value::as_bool) != Some(true) {
        return Err(format!("wallet did not sign the funding tx: {signed}"));
    }
    let hex = signed
        .get("hex")
        .and_then(Value::as_str)
        .ok_or("signed hex")?
        .to_string();
    let tx: Transaction = deserialize(&hex::decode(&hex).map_err(|err| err.to_string())?)
        .map_err(|err| err.to_string())?;
    let vout = tx
        .output
        .iter()
        .position(|output| {
            output.value.to_sat() == sats && output.script_pubkey.as_bytes() == script.as_bytes()
        })
        .ok_or("funded tx is missing the requested output")? as u32;
    let txid_text = node
        .call(false, "sendrawtransaction", json!([hex]))?
        .as_str()
        .ok_or("funding txid")?
        .to_string();
    let txid = Txid::from_str(&txid_text).map_err(|err| err.to_string())?;
    if tx.compute_txid() != txid {
        return Err("funding txid mismatch".into());
    }
    Ok(OutPoint { txid, vout })
}

fn btc(sats: u64) -> String {
    format!("{}.{:08}", sats / 100_000_000, sats % 100_000_000)
}

fn canonical_outpoint(txid: Txid, vout: u32) -> [u8; 36] {
    let mut out = [0u8; 36];
    out[..32].copy_from_slice(&txid.to_byte_array());
    out[32..].copy_from_slice(&vout.to_le_bytes());
    out
}

fn consignment_bytes<const TRANSFER: bool>(
    consignment: &Consignment<TRANSFER>,
) -> Result<Vec<u8>, String> {
    Ok(consignment
        .to_strict_serialized::<MAX_CONSIGNMENT>()
        .map_err(|err| err.to_string())?
        .release())
}

fn load_consignment<const TRANSFER: bool>(bytes: &[u8]) -> Result<Consignment<TRANSFER>, String> {
    let confined = Confined::<Vec<u8>, 0, MAX_CONSIGNMENT>::try_from(bytes.to_vec())
        .map_err(|err| format!("consignment bounds: {err:?}"))?;
    Consignment::<TRANSFER>::from_strict_serialized::<MAX_CONSIGNMENT>(confined)
        .map_err(|err| err.to_string())
}

fn report(status: &Status) -> String {
    let mut out = format!("{:#}", status);
    let mut rows: Vec<_> = status
        .tx_ord_map
        .iter()
        .map(|(txid, ord)| (txid.to_string(), format!("{ord:?}")))
        .collect();
    rows.sort();
    out.push_str("tx_ord_map:\n");
    for (txid, ord) in rows {
        let _ = writeln!(out, "{txid} {ord}");
    }
    out
}

fn validate_one<const TRANSFER: bool>(bytes: &[u8], types: &TypeSystem) -> Result<String, String> {
    let consignment = load_consignment::<TRANSFER>(bytes)?;
    let resolver = chain::ElectrumResolver::open()?;
    let config = ValidationConfig {
        chain_net: chain::chain_net()?,
        safe_height: None,
        trusted_typesystem: types.clone(),
        build_opouts_dag: false,
    };
    match consignment.validate(&resolver, &config) {
        Ok(valid) => Ok(report(valid.validation_status())),
        Err(err) => Ok(format!("ERROR {err:?}")),
    }
}

fn dual_validate<const TRANSFER: bool>(bytes: &[u8], types: &TypeSystem) -> Result<String, String> {
    let first = validate_one::<TRANSFER>(bytes, types)?;
    let second = validate_one::<TRANSFER>(bytes, types)?;
    if first != second {
        return Err(format!(
            "validator mismatch\n--- 1 ---\n{first}\n--- 2 ---\n{second}"
        ));
    }
    Ok(first)
}

fn rights_text(stock: &Stock, contract_id: rgbstd::ContractId, outpoint: OutPoint) -> String {
    match stock.contract_assignments_for(contract_id, [outpoint]) {
        Ok(map) if map.is_empty() => "no assignment on outpoint".into(),
        Ok(map) => {
            let mut lines = Vec::new();
            for (seal, states) in map {
                for (opout, state) in states {
                    lines.push(format!("seal {seal:?} opout {opout} state {state:?}"));
                }
            }
            lines.sort();
            lines.join(" | ")
        }
        Err(err) => format!("assignments error: {err}"),
    }
}

fn all_rights(stock: &Stock, contract_id: rgbstd::ContractId) -> String {
    match stock.contract_state(contract_id) {
        Ok(state) => {
            let mut lines = Vec::new();
            for item in state.rights_all() {
                lines.push(format!(
                    "opout {} seal {:?} witness {:?}",
                    item.opout, item.seal, item.witness
                ));
            }
            lines.sort();
            if lines.is_empty() {
                "no rights".into()
            } else {
                lines.join(" | ")
            }
        }
        Err(err) => format!("state error: {err}"),
    }
}

struct ChainResult {
    text: String,
    pass: bool,
}

fn broadcast_commit(
    log: &mut Log,
    node: &chain::Node,
    stock: &mut Stock,
    types: &TypeSystem,
    spent: &spend::CommittedSpend,
    label: &str,
) -> Result<ChainResult, String> {
    let sent = node.raw(false, "sendrawtransaction", json!([spent.hex]))?;
    if sent.get("error").is_some_and(|err| !err.is_null()) {
        let text = format!("broadcast {sent}");
        return Ok(ChainResult { pass: false, text });
    }
    let height = node.mine(1)?;
    let chain_text = match consume_and_validate(log, node, stock, types, spent, label) {
        Ok(text) => text,
        Err(err) => format!("consume error {err}"),
    };
    let pass =
        chain_text.contains("validators_agree true") && chain_text.contains("Consignment is valid");
    Ok(ChainResult {
        text: format!("mined {height} tx {} {chain_text}", spent.txid),
        pass,
    })
}

fn consume_and_validate(
    log: &mut Log,
    node: &chain::Node,
    stock: &mut Stock,
    types: &TypeSystem,
    spent: &spend::CommittedSpend,
    label: &str,
) -> Result<String, String> {
    let consume_note =
        match stock.consume_fascia(spent.fascia.clone(), chain::ElectrumResolver::open()?) {
            Ok(()) => "consume mined".to_string(),
            Err(err) => {
                stock
                    .consume_fascia(spent.fascia.clone(), TentativeOrd)
                    .map_err(|fallback| format!("consume mined: {err}; tentative: {fallback}"))?;
                let update =
                    stock.update_witnesses(chain::ElectrumResolver::open()?, 1, vec![spent.txid]);
                format!("consume tentative after {err}; update {update:?}")
            }
        };
    let next = OutputSeal::with(spent.txid, 1u32);
    let transfer = stock
        .transfer_from_fascia(
            contract_of(&spent.fascia)?,
            [next],
            &[] as &[SecretSeal],
            [spent.transition_id],
            &spent.fascia,
        )
        .map_err(|err| err.to_string())?;
    let bytes = consignment_bytes(&transfer)?;
    log.write(&format!("{label}-transfer.strict"), &bytes)?;
    let report = dual_validate::<true>(&bytes, types).unwrap_or_else(|err| format!("ERROR {err}"));
    log.write(
        format!("{label}-validators.txt").as_str(),
        report.as_bytes(),
    )?;
    let agree = !report.starts_with("ERROR") && !report.contains("validator mismatch");
    let _ = node;
    Ok(format!(
        "{consume_note} validators_agree {agree} {}",
        one_line(&report)
    ))
}

fn contract_of(fascia: &Fascia) -> Result<rgbstd::ContractId, String> {
    fascia
        .bundles()
        .keys()
        .next()
        .copied()
        .ok_or_else(|| "fascia has no contract".into())
}

fn relative_lock(node: &chain::Node, hex: &str) -> Result<String, String> {
    let start = node.height()?;
    let early = node.raw(false, "sendrawtransaction", json!([hex]))?;
    let early_error = early.get("error").filter(|value| !value.is_null());
    if early_error.is_none() {
        return Ok(format!("FAIL accepted at height {start}: {early}"));
    }
    let early_text = early_error.unwrap().to_string();
    let mut last_reject = String::new();
    let mut allowed_at = None;
    for _ in 0..24 {
        let height = node.height()?;
        let accept = node.call(false, "testmempoolaccept", json!([[hex]]))?;
        let item = accept
            .as_array()
            .and_then(|items| items.first())
            .ok_or("testmempoolaccept")?;
        let allowed = item
            .get("allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if allowed {
            allowed_at = Some(height);
            break;
        }
        last_reject = item
            .get("reject-reason")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        node.mine(1)?;
    }
    let Some(allowed_at) = allowed_at else {
        return Ok(format!(
            "FAIL never allowed start {start} early {early_text} last {last_reject}"
        ));
    };
    let broadcast_at = node.height()?;
    let sent = node.call(false, "sendrawtransaction", json!([hex]))?;
    let mined = node.mine(1)?;
    Ok(format!(
        "start {start} early {early_text} last_reject {last_reject} allowed_at {allowed_at} broadcast_at {broadcast_at} mined {mined} tx {sent}"
    ))
}

fn finish_rest(
    log: &mut Log,
    node: &chain::Node,
    stock: &mut Stock,
    prepared: &schema::IdentitySchema,
    genesis_script: &o2a_demo_core::SealScript,
    fee: &spend::LocalKey,
    keys: &o2a_demo_core::DemoKeys,
    funded: &Funded,
    seal_r: &Issued,
    seal_c: &Issued,
    seal_d1: &Issued,
    seal_d2: &Issued,
) -> Result<(), String> {
    let _ = (seal_r, prepared);
    let plain = spend::plain_spend(
        seal_c.outpoint,
        Amount::from_sat(120_000),
        genesis_script,
        keys.seal_controller_0,
        fee.script.clone(),
        Amount::from_sat(105_000),
    );
    match plain {
        Ok((_, hex_tx)) => {
            log.write("c6-plain.hex", hex_tx.as_bytes())?;
            let sent = node.raw(false, "sendrawtransaction", json!([hex_tx]))?;
            if sent.get("error").is_some_and(|err| !err.is_null()) {
                log.case(
                    "C6",
                    "plain spend leaves the previous RGB validation without an error",
                    &sent.to_string(),
                    false,
                );
            } else {
                let height = node.mine(1)?;
                let again = dual_validate::<false>(&seal_c.bytes, &prepared.types)
                    .unwrap_or_else(|err| format!("ERROR {err}"));
                let bitcoin = chain_spent(node, seal_c.outpoint);
                let rights = all_rights(stock, seal_c.contract_id);
                let pass = !again.contains("ERROR") && !again.contains("validator mismatch");
                log.case(
                    "C6",
                    "plain spend of a seal with no commitment; record the 0.11.1 validation text",
                    &format!(
                        "mined {height} bitcoin {bitcoin} rights {} report {}",
                        one_line(&rights),
                        one_line(&again)
                    ),
                    pass,
                );
            }
        }
        Err(err) => log.case("C6", "plain spend", &err, false),
    }

    let both_unspent = seal_d1.valid && seal_d2.valid && seal_d1.digest_hex == seal_d2.digest_hex;
    let move_d2 = schema::right_transition(
        prepared,
        seal_d2.contract_id,
        "rotate",
        seal_d2.opout,
        AllocatedState::Void,
        1,
        BLINDING,
        14,
    );
    match move_d2 {
        Ok(transition) => {
            let spent = spend::commit_opret(
                seal_d2.outpoint,
                Amount::from_sat(130_000),
                genesis_script,
                spend::LeafSpend::Controller {
                    key: keys.seal_controller_0,
                },
                funded.outs["fee7"],
                Amount::from_sat(62_000),
                &fee.keypair,
                ScriptBuf::from_bytes(genesis_script.script_pubkey.clone()),
                Amount::from_sat(NEXT_SATS),
                fee.script.clone(),
                Amount::from_sat(47_000),
                u32::MAX,
                transition,
            );
            match spent {
                Ok(spent) => {
                    log.write("c7-contract2.hex", spent.hex.as_bytes())?;
                    let broadcast = node.raw(false, "sendrawtransaction", json!([spent.hex]))?;
                    if broadcast.get("error").is_some_and(|err| !err.is_null()) {
                        log.case(
                            "C7",
                            "both contracts validate while the seal is unspent; record contract 1 after a contract-2 spend",
                            &format!("unspent {both_unspent} broadcast {broadcast}"),
                            false,
                        );
                    } else {
                        let height = node.mine(1)?;
                        let _ =
                            consume_and_validate(log, node, stock, &prepared.types, &spent, "c7");
                        let bitcoin = chain_spent(node, seal_d1.outpoint);
                        let again = dual_validate::<false>(&seal_d1.bytes, &prepared.types)
                            .unwrap_or_else(|err| format!("ERROR {err}"));
                        let rights = all_rights(stock, seal_d1.contract_id);
                        let on_spent = rights_text(stock, seal_d1.contract_id, seal_d1.outpoint);
                        log.case(
                            "C7",
                            "both contracts validate on the same unspent outpoint and the same O2A digest; record contract 1 after only contract 2 is spent",
                            &format!(
                                "unspent_both {both_unspent} same_digest {} ids {} {} mined {height} bitcoin {bitcoin} contract1_on_outpoint {} contract1_rights {} contract1_report {}",
                                seal_d1.digest_hex == seal_d2.digest_hex,
                                seal_d1.contract_id,
                                seal_d2.contract_id,
                                one_line(&on_spent),
                                one_line(&rights),
                                one_line(&again)
                            ),
                            both_unspent,
                        );
                    }
                }
                Err(err) => log.case("C7", "contract 2 spend", &err, false),
            }
        }
        Err(err) => log.case("C7", "contract 2 transition", &err, false),
    }
    Ok(())
}

fn chain_spent(node: &chain::Node, outpoint: OutPoint) -> String {
    match node.call(
        false,
        "gettxout",
        json!([outpoint.txid.to_string(), outpoint.vout, false]),
    ) {
        Ok(Value::Null) => "spent".into(),
        Ok(value) => format!(
            "unspent confirmations {}",
            value
                .get("confirmations")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        Err(err) => format!("gettxout error {err}"),
    }
}

fn one_line(text: &str) -> String {
    text.replace('\n', " | ")
}
