//! Maintained RGB 0.11.1 regtest lineage and the stage commands.
//!
//! `plan`, `genesis`, `claim`, and `verify` follow the active profile.
//! The operator's wallet funds the seal. This module does not broadcast that payment.
//!
//! Spend scripts come from decoded signed objects. This module does not import
//! the archived 0.12 adapter.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use o2a_demo_core::{
    controller_rotation_with, decode_identity_state, demo_entity_index, demo_genesis_state,
    demo_keys, encode_resulting_state, entity_id, evaluate_lineage, evaluate_name_claim,
    format_lineage_report, genesis_for, genesis_state_from, genesis_with, inclusion_matches,
    key_id, keys_for, official_name_claim, official_name_nonce, official_name_of,
    recovery_authorizations, seal_for_state, state_id, state_named_by_signed, verify,
    ClaimAuthorization, ControllerEntry, CurrentSealView, DemoKey, DemoKeys, InclusionProof,
    LineageEvidence, ResultingState, SealBinding, SealFact, SignedObject, NUMS_X,
};
#[cfg(test)]
use rgbstd::bitcoin::absolute::LockTime;
use rgbstd::bitcoin::consensus::encode::deserialize;
#[cfg(test)]
use rgbstd::bitcoin::consensus::encode::serialize;
use rgbstd::bitcoin::hashes::Hash;
#[cfg(test)]
use rgbstd::bitcoin::transaction::Version;
use rgbstd::bitcoin::{Address, Amount, OutPoint, ScriptBuf, Transaction, Txid};
#[cfg(test)]
use rgbstd::bitcoin::{Sequence, TxIn, TxOut, Witness};
use rgbstd::containers::ConsignmentExt;
use rgbstd::contract::AllocatedState;
use rgbstd::persistence::Stock;
use rgbstd::{Operation, Opout};
use serde_json::{json, Value};

const BLINDING: u64 = 0x0A11_0110;
const SEAL_SATS: u64 = 200_000;
const FEE_SATS: u64 = 80_000;
const NEXT_SATS: u64 = 100_000;
const CHANGE_SATS: u64 = 165_000;
const PLAIN_SATS: u64 = 150_000;
const PLAIN_PAY: u64 = 135_000;
/// Recorded regtest lineage depth. The profile's required depth stays separate
/// so this evidence timing does not move.
const DEPTH: u32 = 2;

struct Funded {
    point: OutPoint,
    sequences: Vec<u32>,
    hex: String,
}

pub fn plan(authorize: bool) -> Result<(), String> {
    let active = crate::profile::load()?;
    // The session lock runs before any key derivation.
    crate::profile::begin_cli(&active, crate::profile::Operation::Plan, authorize)?;
    require_plan_seed(active.kind)?;
    let keys = keys_for(active.coin_type);
    let state = genesis_state_from(&keys, [0u8; 36]);
    let seal = seal_for_state(&state).map_err(|err| err.to_string())?;
    let address = Address::from_script(
        ScriptBuf::from_bytes(seal.script_pubkey.clone()).as_script(),
        active.bitcoin,
    )
    .map_err(|err| err.to_string())?;
    if let Ok(dir) = std::env::var("RGB011_EVIDENCE") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let document = json!({
            "network": active.kind.label(),
            "seal_address": address.to_string(),
            "script_pubkey": hex::encode(&seal.script_pubkey),
            "required_depth": active.required_depth,
        });
        let body = serde_json::to_string_pretty(&document).map_err(|err| err.to_string())? + "\n";
        std::fs::write(dir.join("plan.json"), body).map_err(|err| err.to_string())?;
    }
    println!("entity_index={}", demo_entity_index());
    println!("network={}", active.network_byte);
    println!("delay_blocks={}", state.recovery.delay_blocks);
    println!("threshold={}", state.recovery.threshold);
    let policy = core_descriptor(&state);
    println!("address={address}");
    println!("policy={policy}");
    println!("descriptor={policy}");
    println!("script_pubkey={}", hex::encode(&seal.script_pubkey));
    println!("root_xonly={}", hex::encode(keys.root.xonly));
    println!(
        "controller_0_xonly={}",
        hex::encode(keys.controller_0.xonly)
    );
    println!(
        "controller_1_xonly={}",
        hex::encode(keys.controller_1.xonly)
    );
    println!("recovery_0_xonly={}", hex::encode(keys.recovery_0.xonly));
    println!("recovery_1_xonly={}", hex::encode(keys.recovery_1.xonly));
    println!("recovery_2_xonly={}", hex::encode(keys.recovery_2.xonly));
    println!(
        "seal_controller_0_xonly={}",
        hex::encode(keys.seal_controller_0.xonly)
    );
    println!(
        "seal_controller_1_xonly={}",
        hex::encode(keys.seal_controller_1.xonly)
    );
    println!(
        "seal_recovery_0_xonly={}",
        hex::encode(keys.seal_recovery_0.xonly)
    );
    println!(
        "seal_recovery_1_xonly={}",
        hex::encode(keys.seal_recovery_1.xonly)
    );
    println!(
        "seal_recovery_2_xonly={}",
        hex::encode(keys.seal_recovery_2.xonly)
    );
    Ok(())
}

/// Keyless chain pre-flight. It consumes the public plan and observes the
/// operator-funded outpoint; it never derives a key and never broadcasts.
pub fn preflight(authorize: bool, seal: Option<&str>, json_out: bool) -> Result<(), String> {
    let active = crate::profile::load()?;
    crate::profile::begin_cli(&active, crate::profile::Operation::Preflight, authorize)?;
    if !json_out {
        return Err("preflight requires --json".into());
    }
    let outpoint_text = seal
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("preflight requires --seal <outpoint>")?;
    let outpoint = parse_outpoint(outpoint_text)?;
    let dir =
        PathBuf::from(std::env::var("RGB011_EVIDENCE").map_err(|_| "RGB011_EVIDENCE is required")?);
    let plan: Value = serde_json::from_slice(
        &std::fs::read(dir.join("plan.json")).map_err(|err| format!("plan.json: {err}"))?,
    )
    .map_err(|err| format!("plan.json: {err}"))?;
    let planned_network = plan
        .get("network")
        .and_then(Value::as_str)
        .ok_or("plan.json network is missing")?;
    if planned_network != active.kind.label() {
        return Err(format!(
            "plan network {planned_network} does not match {}",
            active.kind.label()
        ));
    }
    let planned_address = plan
        .get("seal_address")
        .and_then(Value::as_str)
        .ok_or("plan.json seal_address is missing")?;
    let planned_script = hex::decode(
        plan.get("script_pubkey")
            .and_then(Value::as_str)
            .ok_or("plan.json script_pubkey is missing")?,
    )
    .map_err(|err| format!("plan.json script_pubkey: {err}"))?;
    let planned_depth = plan
        .get("required_depth")
        .and_then(Value::as_u64)
        .ok_or("plan.json required_depth is missing")? as u32;
    if planned_depth != active.required_depth {
        return Err("plan.json required_depth does not match the network profile".into());
    }

    let node = super::chain::Node::connect_wallet(false)?;
    let observed = node.observe_funding(&outpoint.txid.to_string(), outpoint.vout)?;
    if !observed.output_exists {
        return Err("planned seal output does not exist on the selected chain".into());
    }
    if observed.output_script != planned_script {
        return Err("funded output script does not match plan.json".into());
    }
    let chain_address = Address::from_script(
        ScriptBuf::from_bytes(observed.output_script.clone()).as_script(),
        active.bitcoin,
    )
    .map_err(|err| err.to_string())?
    .to_string();
    if chain_address != planned_address {
        return Err("funded output address does not match plan.json".into());
    }
    if !inclusion_matches(&observed.inclusion) || observed.txout_proof.is_empty() {
        return Err("funding inclusion proof is missing or does not match its header".into());
    }
    let node_height = node.height()?;
    let mut electrs = super::chain::ElectrumResolver::open()?;
    let electrs_height = electrs.tip()?;
    let document = json!({
        "network": active.kind.label(),
        "node_height": node_height,
        "electrs_height": electrs_height,
        "seal_address": chain_address,
        "seal_outpoint": outpoint_text,
        "funding_confirmations": observed.confirmations,
        "required_depth": active.required_depth,
        "unspent": observed.unspent,
    });
    let body = serde_json::to_string_pretty(&document).map_err(|err| err.to_string())? + "\n";
    std::fs::write(dir.join("preflight.json"), &body).map_err(|err| err.to_string())?;
    print!("{body}");
    Ok(())
}

pub fn run(authorize: bool) -> Result<(), String> {
    let active = crate::profile::load()?;
    crate::profile::begin_cli(&active, crate::profile::Operation::Lineage, authorize)?;
    crate::profile::require_regtest_command(&active)?;
    require_seed()?;
    let dir =
        PathBuf::from(std::env::var("RGB011_EVIDENCE").map_err(|_| "RGB011_EVIDENCE is required")?);
    let mut log = super::Log::open(dir)?;
    let node = super::chain::Node::connect()?;
    let h0 = node.height()?;
    if h0 < 101 {
        return Err("mine 101 blocks and let electrs index them before lineage; H0 is the height at this start".into());
    }
    super::chain::wait_electrs(h0)?;
    log.line(&format!("H0 {h0}"));
    log.line(&format!("network {}", active.kind.label()));
    log.line("rgb-protocol 0.11.1 Opret");
    log.line("disposable demo-lineage evidence; no Phase 0 gate closure");
    let mut failed = false;
    let prepared = super::schema::identity_schema();
    let mut stock = Stock::in_memory();
    let fee = super::spend::local_key()?;
    log.line(&format!("fee_xonly {}", hex::encode(fee.xonly)));

    let rbf = replace_non_seal(&node)?;
    let rbf_ok = rbf.contains("txid_changed true");
    record(
        &mut log,
        &mut failed,
        "RBF",
        "a non-seal replaceable payment changes txid",
        &rbf,
        rbf_ok,
    );

    let mut outputs = Vec::new();
    for (label, entity, sats) in [
        ("e11", 11u32, SEAL_SATS),
        ("e12", 12, SEAL_SATS),
        ("e13", 13, SEAL_SATS),
        ("e15", 15, SEAL_SATS),
        ("e16", 16, PLAIN_SATS),
    ] {
        use_entity(entity);
        let state = demo_genesis_state([0u8; 36]);
        let seal = seal_for_state(&state).map_err(|err| err.to_string())?;
        let address = core_address(&node, &state)?;
        log.line(&format!(
            "{label} entity {entity} address {address} descriptor {}",
            core_descriptor(&state)
        ));
        outputs.push((
            label,
            ScriptBuf::from_bytes(seal.script_pubkey),
            sats,
            false,
        ));
    }
    for label in ["fee11", "fee12", "fee13", "fee15"] {
        outputs.push((label, fee.script.clone(), FEE_SATS, false));
    }
    while super::spendable_count(&node)? < outputs.len() {
        node.mine(1)?;
    }
    let mut funded = BTreeMap::new();
    for (label, script, sats, _) in &outputs {
        let item = fund_output(&node, script, *sats, false)?;
        if !sequences_final(&item.sequences) {
            record(
                &mut log,
                &mut failed,
                "FUND",
                "seal funding inputs use sequence 4294967294 or 4294967295",
                &format!("{label} sequences {:?}", item.sequences),
                false,
            );
            return Err(format!("{label} funding signaled replacement"));
        }
        log.line(&format!(
            "fund {label} {}:{} sequences {:?}",
            item.point.txid, item.point.vout, item.sequences
        ));
        log.write(&format!("fund-{label}.hex"), item.hex.as_bytes())?;
        funded.insert(*label, item);
    }
    let cpfp = cpfp_change(&node, &funded["e11"])?;
    let parent = funded["e11"].point.txid.to_string();
    let cpfp_ok =
        cpfp.contains(&format!("parent {parent}")) && cpfp.contains("parent_txid_unchanged true");
    record(
        &mut log,
        &mut failed,
        "CPFP",
        "seal funding stays replaceable=false and a child spends only the change",
        &cpfp,
        cpfp_ok,
    );
    let f_height = node.mine(1)?;
    log.line(&format!("F {f_height}"));
    if f_height <= h0 {
        return Err("funding confirmation is not above H0".into());
    }

    let e11 = issue_entity(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        11,
        &funded["e11"].point,
        super::schema::TS_ISSUE,
        "e11",
    )?;
    let pending = o2a_report(
        &node,
        &e11,
        e11.outpoint,
        None,
        true,
        "Consignment is valid",
    )?;
    let pending_ok = pending.contains("identity_history_state=PENDING_CONFIRMATION");
    record(
        &mut log,
        &mut failed,
        "PENDING",
        "genesis at the confirmation block is PENDING_CONFIRMATION for depth 2",
        &pending,
        pending_ok && e11.rgb_ok,
    );
    node.mine(1)?;
    let current = o2a_report(
        &node,
        &e11,
        e11.outpoint,
        None,
        true,
        "Consignment is valid",
    )?;
    let current_ok = current.contains("identity_history_state=CURRENT");
    record(
        &mut log,
        &mut failed,
        "CURRENT",
        "one more block makes the same genesis CURRENT",
        &current,
        current_ok && e11.rgb_ok,
    );

    let rotated = rotate_entity(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        &fee,
        11,
        &e11,
        &funded["fee11"].point,
        "e11-rotate",
    )?;
    node.mine(1)?;
    let rotate_view = o2a_report(
        &node,
        &rotated.identity,
        rotated.next,
        None,
        true,
        "Consignment is valid",
    )?;
    record(
        &mut log,
        &mut failed,
        "ROTATE",
        "rotation spends the decoded genesis seal and the successor is CURRENT",
        &format!("{} {}", rotated.note, rotate_view),
        rotated.ok && rotate_view.contains("identity_history_state=CURRENT"),
    );

    let e12 = issue_entity(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        12,
        &funded["e12"].point,
        super::schema::TS_ISSUE + 1,
        "e12",
    )?;
    let recovery = recover_entity(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        &fee,
        &e12,
        &funded["fee12"].point,
        f_height,
        "e12-recover",
    )?;
    node.mine(1)?;
    let recover_view = o2a_report(
        &node,
        &recovery.identity,
        recovery.next,
        None,
        true,
        "Consignment is valid",
    )?;
    record(
        &mut log,
        &mut failed,
        "RECOVER",
        "BIP68 recovery is non-final before maturity and the successor is CURRENT",
        &format!("{} {}", recovery.note, recover_view),
        recovery.ok && recover_view.contains("identity_history_state=CURRENT"),
    );

    let shared = same_seal(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        &fee,
        &funded["e13"].point,
        &funded["fee13"].point,
    )?;
    node.mine(1)?;
    record(
        &mut log,
        &mut failed,
        "SAME-SEAL",
        "two entity indexes share one outpoint; the one not continued is SEAL_CLOSED",
        &shared,
        shared.contains("continued CURRENT")
            && shared.contains("other SEAL_CLOSED")
            && shared.contains("rgb_other valid"),
    );

    let reissue = reissue_pair(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        &fee,
        &funded["e15"].point,
        &funded["fee15"].point,
    )?;
    node.mine(1)?;
    record(
        &mut log,
        &mut failed,
        "REISSUE",
        "two contracts on one seal; the contract not continued is SEAL_CLOSED while its consignment still validates",
        &reissue,
        reissue.contains("continued CURRENT") && reissue.contains("other SEAL_CLOSED") && reissue.contains("rgb_other valid"),
    );

    let e16 = issue_entity(
        &mut log,
        &node,
        &mut stock,
        &prepared,
        16,
        &funded["e16"].point,
        super::schema::TS_ISSUE + 6,
        "e16",
    )?;
    let plain = plain_close(&mut log, &node, &e16, &fee)?;
    node.mine(1)?;
    let closed = o2a_report(
        &node,
        &e16,
        e16.outpoint,
        Some(plain.spend),
        false,
        "Consignment is valid",
    )?;
    let again = super::dual_validate::<false>(&e16.bytes, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    let plain_ok = closed.contains("identity_history_state=SEAL_CLOSED_WITHOUT_VALID_TRANSITION")
        && again.contains("Consignment is valid")
        && !again.contains("validator mismatch");
    record(
        &mut log,
        &mut failed,
        "PLAIN",
        "a plain close has no O2A transition; RGB still validates the genesis consignment",
        &format!("{} {} {}", plain.note, closed, super::one_line(&again)),
        plain_ok,
    );

    let reorg = reorg_above(&node, h0)?;
    record(
        &mut log,
        &mut failed,
        "REORG",
        "invalidateblock and reconsiderblock stay above H0",
        &reorg,
        reorg.contains("above_h0 true") && reorg.contains("restored true"),
    );

    log.line("lineage_finished");
    if failed {
        return Err("one or more lineage cases failed".into());
    }
    Ok(())
}

struct Identity {
    entity: [u8; 32],
    genesis: SignedObject,
    state: ResultingState,
    outpoint: OutPoint,
    contract_id: rgbstd::ContractId,
    opout: Opout,
    bytes: Vec<u8>,
    rgb_ok: bool,
}

struct Moved {
    identity: Identity,
    next: OutPoint,
    note: String,
    ok: bool,
}

fn issue_entity(
    log: &mut super::Log,
    node: &super::chain::Node,
    stock: &mut Stock,
    prepared: &super::schema::IdentitySchema,
    entity_index: u32,
    outpoint: &OutPoint,
    timestamp: i64,
    label: &str,
) -> Result<Identity, String> {
    use_entity(entity_index);
    let keys = demo_keys();
    let canonical = super::canonical_outpoint(outpoint.txid, outpoint.vout);
    let state = demo_genesis_state(canonical);
    let signed = genesis_with(keys.root, &state);
    verify(&signed).map_err(|err| err.to_string())?;
    let decoded =
        state_named_by_signed(&[&signed.payload], &canonical).map_err(|err| err.to_string())?;
    if decoded != state {
        return Err(format!(
            "{label} decoded state differs from the signed genesis"
        ));
    }
    let seal = seal_for_state(&decoded).map_err(|err| err.to_string())?;
    let observed = output_script(node, &outpoint.txid.to_string(), outpoint.vout)?;
    if observed != seal.script_pubkey {
        return Err(format!(
            "{label} chain script does not match the decoded seal"
        ));
    }
    write_object(log, &format!("{label}-genesis.o2a"), &signed)?;
    let entity = entity_id(&signed.payload);
    log.line(&format!(
        "{label} entity_index {entity_index} entity {} state {}",
        hex::encode(entity),
        hex::encode(state_id(&entity, &encode_resulting_state(&state)))
    ));
    let issued = super::schema::issue_on(
        prepared,
        super::chain::chain_net()?,
        signed.digest,
        *outpoint,
        BLINDING,
        timestamp,
    )?;
    let contract_id = ConsignmentExt::contract_id(&*issued);
    let opout = Opout::new(
        ConsignmentExt::genesis(&*issued).id(),
        super::schema::OS_IDENTITY,
        0,
    );
    let bytes = super::consignment_bytes(&*issued)?;
    log.write(&format!("{label}-genesis.strict"), &bytes)?;
    let report = super::dual_validate::<false>(&bytes, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    log.write(&format!("{label}-validators.txt"), report.as_bytes())?;
    let rgb_ok =
        report.starts_with("Consignment is valid") && !report.contains("validator mismatch");
    let _ = stock.import_contract(
        issued.into_valid_contract(),
        super::chain::ElectrumResolver::open()?,
    );
    Ok(Identity {
        entity,
        genesis: signed,
        state,
        outpoint: *outpoint,
        contract_id,
        opout,
        bytes,
        rgb_ok,
    })
}

fn rotate_entity(
    log: &mut super::Log,
    node: &super::chain::Node,
    stock: &mut Stock,
    prepared: &super::schema::IdentitySchema,
    fee: &super::spend::LocalKey,
    entity_index: u32,
    identity: &Identity,
    fee_point: &OutPoint,
    label: &str,
) -> Result<Moved, String> {
    use_entity(entity_index);
    let keys = demo_keys();
    let spent_state = decode_named(&identity.genesis, &identity.outpoint)?;
    let seal = seal_for_state(&spent_state).map_err(|err| err.to_string())?;
    let draft = rotated_state(&spent_state, &identity.genesis, &keys, [0u8; 36])?;
    let next_seal_script = seal_for_state(&draft).map_err(|err| err.to_string())?;
    let transition = super::schema::right_transition(
        prepared,
        identity.contract_id,
        "rotate",
        identity.opout,
        AllocatedState::Void,
        1,
        BLINDING,
        21,
    )?;
    let spent = super::spend::commit_opret(
        identity.outpoint,
        Amount::from_sat(SEAL_SATS),
        &seal,
        super::spend::LeafSpend::Controller {
            key: seal_key(&spent_state, &keys)?,
        },
        *fee_point,
        Amount::from_sat(FEE_SATS),
        &fee.keypair,
        ScriptBuf::from_bytes(next_seal_script.script_pubkey.clone()),
        Amount::from_sat(NEXT_SATS),
        fee.script.clone(),
        Amount::from_sat(CHANGE_SATS),
        u32::MAX,
        transition,
    )?;
    log.write(&format!("{label}.hex"), spent.hex.as_bytes())?;
    let next = OutPoint {
        txid: spent.txid,
        vout: 1,
    };
    let next_state = rotated_state(
        &spent_state,
        &identity.genesis,
        &keys,
        super::canonical_outpoint(next.txid, next.vout),
    )?;
    let next_bytes = controller_rotation_with(
        keys.controller_0,
        identity.entity,
        state_id(&identity.entity, &encode_resulting_state(&spent_state)),
        &next_state,
    );
    verify(&next_bytes).map_err(|err| err.to_string())?;
    let next_decoded = decode_identity_state(&next_bytes.payload).map_err(|err| err.to_string())?;
    write_object(log, &format!("{label}.o2a"), &next_bytes)?;
    let sent = node.raw(false, "sendrawtransaction", json!([spent.hex]))?;
    if sent.get("error").is_some_and(|err| !err.is_null()) {
        return Ok(Moved {
            identity: successor(identity, &next_decoded, next),
            next,
            note: format!("broadcast {sent}"),
            ok: false,
        });
    }
    let height = node.mine(1)?;
    let chain_text = super::consume_and_validate(log, node, stock, &prepared.types, &spent, label)
        .unwrap_or_else(|err| format!("consume error {err}"));
    Ok(Moved {
        identity: successor(identity, &next_decoded, next),
        next,
        note: format!("mined {height} {chain_text}"),
        ok: chain_text.contains("validators_agree true")
            && chain_text.contains("Consignment is valid"),
    })
}

fn recover_entity(
    log: &mut super::Log,
    node: &super::chain::Node,
    stock: &mut Stock,
    prepared: &super::schema::IdentitySchema,
    fee: &super::spend::LocalKey,
    identity: &Identity,
    fee_point: &OutPoint,
    funded_at: u32,
    label: &str,
) -> Result<Moved, String> {
    use_entity(12);
    let keys = demo_keys();
    let spent_state = decode_named(&identity.genesis, &identity.outpoint)?;
    let seal = seal_for_state(&spent_state).map_err(|err| err.to_string())?;
    let mut draft = spent_state.clone();
    draft.sequence = 1;
    draft.previous_state = Some(state_id(
        &identity.entity,
        &encode_resulting_state(&spent_state),
    ));
    draft.previous_seal = Some(super::canonical_outpoint(
        identity.outpoint.txid,
        identity.outpoint.vout,
    ));
    draft.next_seal = [0u8; 36];
    let next_script = seal_for_state(&draft).map_err(|err| err.to_string())?;
    let transition = super::schema::right_transition(
        prepared,
        identity.contract_id,
        "recover",
        identity.opout,
        AllocatedState::Void,
        1,
        BLINDING,
        22,
    )?;
    let spent = super::spend::commit_opret(
        identity.outpoint,
        Amount::from_sat(SEAL_SATS),
        &seal,
        super::spend::LeafSpend::Recovery {
            keys: [
                keys.seal_recovery_0,
                keys.seal_recovery_1,
                keys.seal_recovery_2,
            ],
        },
        *fee_point,
        Amount::from_sat(FEE_SATS),
        &fee.keypair,
        ScriptBuf::from_bytes(next_script.script_pubkey),
        Amount::from_sat(NEXT_SATS),
        fee.script.clone(),
        Amount::from_sat(CHANGE_SATS),
        spent_state.recovery.delay_blocks,
        transition,
    )?;
    log.write(&format!("{label}.hex"), spent.hex.as_bytes())?;
    let next = OutPoint {
        txid: spent.txid,
        vout: 1,
    };
    draft.next_seal = super::canonical_outpoint(next.txid, next.vout);
    let auths = recovery_authorizations(
        identity.entity,
        state_id(&identity.entity, &encode_resulting_state(&spent_state)),
        &spent_state.recovery,
        funded_at + spent_state.recovery.delay_blocks,
        &draft,
        &[keys.recovery_0, keys.recovery_2],
    )
    .map_err(|err| err.to_string())?;
    for (index, auth) in auths.iter().enumerate() {
        verify(auth).map_err(|err| err.to_string())?;
        write_object(log, &format!("{label}-{index}.o2a"), auth)?;
    }
    let next_decoded = decode_identity_state(&auths[0].payload).map_err(|err| err.to_string())?;
    if next_decoded.next_seal != draft.next_seal {
        return Err("recovery object does not name the successor seal".into());
    }
    let gate = super::relative_lock(node, &spent.hex)?;
    let allowed = field_u32(&gate, "allowed_at");
    let mined = field_u32(&gate, "mined");
    let mature = allowed.is_some_and(|height| {
        height
            >= funded_at
                .saturating_add(spent_state.recovery.delay_blocks)
                .saturating_sub(1)
    }) && mined.is_some_and(|height| {
        height >= funded_at.saturating_add(spent_state.recovery.delay_blocks)
    });
    let bip68 = gate.contains("non-BIP68-final") && !gate.contains("FAIL") && mature;
    let chain_text = if bip68 {
        super::consume_and_validate(log, node, stock, &prepared.types, &spent, label)
            .unwrap_or_else(|err| format!("consume error {err}"))
    } else {
        String::new()
    };
    Ok(Moved {
        identity: successor(identity, &next_decoded, next),
        next,
        note: format!("funded_at {funded_at} {gate} {chain_text}"),
        ok: bip68 && chain_text.contains("validators_agree true"),
    })
}

fn same_seal(
    log: &mut super::Log,
    node: &super::chain::Node,
    stock: &mut Stock,
    prepared: &super::schema::IdentitySchema,
    fee: &super::spend::LocalKey,
    outpoint: &OutPoint,
    fee_point: &OutPoint,
) -> Result<String, String> {
    use_entity(13);
    let keys13 = demo_keys();
    let canonical = super::canonical_outpoint(outpoint.txid, outpoint.vout);
    let state = demo_genesis_state(canonical);
    let first = genesis_with(keys13.root, &state);
    use_entity(14);
    let second = genesis_with(demo_keys().root, &state);
    let id13 = entity_id(&first.payload);
    let id14 = entity_id(&second.payload);
    if id13 == id14 {
        return Err("same-seal geneses produced one EntityID".into());
    }
    write_object(log, "e13-genesis.o2a", &first)?;
    write_object(log, "e14-genesis.o2a", &second)?;
    verify(&first).map_err(|err| err.to_string())?;
    verify(&second).map_err(|err| err.to_string())?;
    let issued13 = super::schema::issue_on(
        prepared,
        super::chain::chain_net()?,
        first.digest,
        *outpoint,
        BLINDING,
        super::schema::TS_ISSUE + 2,
    )?;
    let issued14 = super::schema::issue_on(
        prepared,
        super::chain::chain_net()?,
        second.digest,
        *outpoint,
        BLINDING,
        super::schema::TS_ISSUE + 3,
    )?;
    let id_contract_13 = ConsignmentExt::contract_id(&*issued13);
    let id_contract_14 = ConsignmentExt::contract_id(&*issued14);
    let opout13 = Opout::new(
        ConsignmentExt::genesis(&*issued13).id(),
        super::schema::OS_IDENTITY,
        0,
    );
    let bytes13 = super::consignment_bytes(&*issued13)?;
    let bytes14 = super::consignment_bytes(&*issued14)?;
    log.write("e13-genesis.strict", &bytes13)?;
    log.write("e14-genesis.strict", &bytes14)?;
    let rgb13 = super::dual_validate::<false>(&bytes13, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    let rgb14 = super::dual_validate::<false>(&bytes14, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    let _ = (
        rgb14,
        stock.import_contract(
            issued13.into_valid_contract(),
            super::chain::ElectrumResolver::open()?,
        ),
    );
    let continued = Identity {
        entity: id13,
        genesis: first.clone(),
        state: state.clone(),
        outpoint: *outpoint,
        contract_id: id_contract_13,
        opout: opout13,
        bytes: bytes13,
        rgb_ok: rgb13.starts_with("Consignment is valid"),
    };
    let moved = rotate_entity(
        log,
        node,
        stock,
        prepared,
        fee,
        13,
        &continued,
        fee_point,
        "e13-rotate",
    )?;
    node.mine(1)?;
    let rgb_other = super::dual_validate::<false>(&bytes14, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    let other_identity = Identity {
        entity: id14,
        genesis: second,
        state,
        outpoint: *outpoint,
        contract_id: id_contract_14,
        opout: continued.opout,
        bytes: bytes14,
        rgb_ok: rgb_other.starts_with("Consignment is valid"),
    };
    let closed = o2a_report(
        node,
        &other_identity,
        *outpoint,
        Some(moved.next.txid),
        false,
        "Consignment is valid",
    )?;
    let open = o2a_report(
        node,
        &moved.identity,
        moved.next,
        None,
        true,
        "Consignment is valid",
    )?;
    Ok(format!(
        "ids {} {} distinct {} contracts {} {} rgb_other {} continued {} other {} note {}",
        hex::encode(id13),
        hex::encode(id14),
        id13 != id14,
        id_contract_13,
        id_contract_14,
        if rgb_other.contains("Consignment is valid") && !rgb_other.contains("validator mismatch") {
            "valid"
        } else {
            "invalid"
        },
        if open.contains("identity_history_state=CURRENT") {
            "CURRENT"
        } else {
            "not-current"
        },
        if closed.contains("SEAL_CLOSED_WITHOUT_VALID_TRANSITION") {
            "SEAL_CLOSED"
        } else {
            "not-closed"
        },
        super::one_line(&moved.note)
    ))
}

fn reissue_pair(
    log: &mut super::Log,
    node: &super::chain::Node,
    stock: &mut Stock,
    prepared: &super::schema::IdentitySchema,
    fee: &super::spend::LocalKey,
    outpoint: &OutPoint,
    fee_point: &OutPoint,
) -> Result<String, String> {
    let closed_issue = issue_entity(
        log,
        node,
        stock,
        prepared,
        15,
        outpoint,
        super::schema::TS_ISSUE + 4,
        "e15a",
    )?;
    let second = super::schema::issue_on(
        prepared,
        super::chain::chain_net()?,
        closed_issue.genesis.digest,
        *outpoint,
        BLINDING,
        super::schema::TS_ISSUE + 5,
    )?;
    let second_id = ConsignmentExt::contract_id(&*second);
    let second_opout = Opout::new(
        ConsignmentExt::genesis(&*second).id(),
        super::schema::OS_IDENTITY,
        0,
    );
    let second_bytes = super::consignment_bytes(&*second)?;
    log.write("e15b-genesis.strict", &second_bytes)?;
    let rgb_second = super::dual_validate::<false>(&second_bytes, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    if second_id == closed_issue.contract_id {
        return Err("reissue produced one contract id".into());
    }
    let _ = stock.import_contract(
        second.into_valid_contract(),
        super::chain::ElectrumResolver::open()?,
    );
    let continued = Identity {
        entity: closed_issue.entity,
        genesis: closed_issue.genesis.clone(),
        state: closed_issue.state.clone(),
        outpoint: *outpoint,
        contract_id: second_id,
        opout: second_opout,
        bytes: second_bytes,
        rgb_ok: rgb_second.starts_with("Consignment is valid"),
    };
    let moved = rotate_entity(
        log,
        node,
        stock,
        prepared,
        fee,
        15,
        &continued,
        fee_point,
        "e15-rotate",
    )?;
    node.mine(1)?;
    let rgb_other = super::dual_validate::<false>(&closed_issue.bytes, &prepared.types)
        .unwrap_or_else(|err| format!("ERROR {err}"));
    let closed = o2a_report(
        node,
        &closed_issue,
        *outpoint,
        Some(moved.next.txid),
        false,
        "Consignment is valid",
    )?;
    let open = o2a_report(
        node,
        &moved.identity,
        moved.next,
        None,
        true,
        "Consignment is valid",
    )?;
    Ok(format!(
        "same_digest true contracts {} {} rgb_second {} rgb_other {} continued {} other {}",
        second_id,
        closed_issue.contract_id,
        if rgb_second.starts_with("Consignment is valid") {
            "valid"
        } else {
            "invalid"
        },
        if rgb_other.contains("Consignment is valid") && !rgb_other.contains("validator mismatch") {
            "valid"
        } else {
            "invalid"
        },
        if open.contains("identity_history_state=CURRENT") {
            "CURRENT"
        } else {
            "not-current"
        },
        if closed.contains("SEAL_CLOSED_WITHOUT_VALID_TRANSITION") {
            "SEAL_CLOSED"
        } else {
            "not-closed"
        }
    ))
}

struct Plain {
    spend: Txid,
    note: String,
}

fn plain_close(
    log: &mut super::Log,
    node: &super::chain::Node,
    identity: &Identity,
    fee: &super::spend::LocalKey,
) -> Result<Plain, String> {
    use_entity(16);
    let keys = demo_keys();
    let spent_state = decode_named(&identity.genesis, &identity.outpoint)?;
    let seal = seal_for_state(&spent_state).map_err(|err| err.to_string())?;
    let (tx, hex_tx) = super::spend::plain_spend(
        identity.outpoint,
        Amount::from_sat(PLAIN_SATS),
        &seal,
        seal_key(&spent_state, &keys)?,
        fee.script.clone(),
        Amount::from_sat(PLAIN_PAY),
    )?;
    log.write("e16-plain.hex", hex_tx.as_bytes())?;
    let sent = node.raw(false, "sendrawtransaction", json!([hex_tx]))?;
    if sent.get("error").is_some_and(|err| !err.is_null()) {
        return Err(format!("plain broadcast {sent}"));
    }
    let height = node.mine(1)?;
    Ok(Plain {
        spend: tx.compute_txid(),
        note: format!("mined {height} tx {}", tx.compute_txid()),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StageObservation {
    text: String,
    state: &'static str,
    bitcoin: &'static str,
    unspent: bool,
    confirmations: u64,
    best_height: u32,
    best_block_hash: String,
}

fn o2a_report(
    node: &super::chain::Node,
    identity: &Identity,
    current: OutPoint,
    spend: Option<Txid>,
    valid_transition: bool,
    rgb: &str,
) -> Result<String, String> {
    Ok(o2a_report_with(node, identity, current, spend, valid_transition, rgb, DEPTH)?.text)
}

fn o2a_report_with(
    node: &super::chain::Node,
    identity: &Identity,
    current: OutPoint,
    spend: Option<Txid>,
    valid_transition: bool,
    rgb: &str,
    depth: u32,
) -> Result<StageObservation, String> {
    let first = o2a_once(node, identity, current, spend, valid_transition, rgb, depth)?;
    let second = o2a_once(node, identity, current, spend, valid_transition, rgb, depth)?;
    if first != second {
        return Err(format!(
            "o2a validators differ\n{}\nbest_block_hash={}\n{}\nbest_block_hash={}",
            first.text, first.best_block_hash, second.text, second.best_block_hash
        ));
    }
    Ok(first)
}

fn tip(node: &super::chain::Node) -> Result<(u32, String), String> {
    let info = node.call(false, "getblockchaininfo", json!([]))?;
    let blocks = info
        .get("blocks")
        .and_then(Value::as_u64)
        .ok_or("best chain height is missing")? as u32;
    let hash = info
        .get("bestblockhash")
        .and_then(Value::as_str)
        .ok_or("best block hash is missing")?
        .to_ascii_lowercase();
    if hash.len() != 64 || !hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("best block hash is not 64 hex characters".into());
    }
    let counted = node.height()?;
    if blocks != counted {
        return Err(format!(
            "tip height {blocks} disagrees with getblockcount {counted}"
        ));
    }
    Ok((blocks, hash))
}

fn spending_txid(
    node: &super::chain::Node,
    outpoint: OutPoint,
    script: &[u8],
) -> Result<Option<Txid>, String> {
    use rgbstd::bitcoin::hashes::sha256;

    let mut digest = sha256::Hash::hash(script).to_byte_array();
    digest.reverse();
    let mut electrs = super::chain::ElectrumResolver::open()?;
    let history = electrs.call(
        "blockchain.scripthash.get_history",
        json!([hex::encode(digest)]),
    )?;
    let entries = history
        .as_array()
        .ok_or("electrs script history is not an array")?;
    for entry in entries {
        let Some(candidate) = entry.get("tx_hash").and_then(Value::as_str) else {
            continue;
        };
        if candidate == outpoint.txid.to_string() {
            continue;
        }
        let (raw, _, _) = node.tx_hex(candidate)?;
        let transaction: Transaction = deserialize(
            &hex::decode(raw).map_err(|err| format!("spending transaction hex: {err}"))?,
        )
        .map_err(|err| format!("spending transaction: {err}"))?;
        if transaction
            .input
            .iter()
            .any(|input| input.previous_output == outpoint)
        {
            return Ok(Some(transaction.compute_txid()));
        }
    }
    Ok(None)
}

fn o2a_once(
    node: &super::chain::Node,
    identity: &Identity,
    current: OutPoint,
    spend: Option<Txid>,
    valid_transition: bool,
    rgb: &str,
    depth: u32,
) -> Result<StageObservation, String> {
    let canonical = super::canonical_outpoint(current.txid, current.vout);
    let payloads = [identity.genesis.payload.as_slice()];
    let state = state_named_by_signed(&payloads, &canonical).or_else(|_| {
        // A successor object names the new seal. Fall back to the carried state
        // when this history's genesis names the previous seal.
        Ok::<_, String>(identity.state.clone())
    })?;
    let expected = if state.next_seal == canonical {
        seal_for_state(&state)
            .map_err(|err| err.to_string())?
            .script_pubkey
    } else {
        seal_for_state(&identity.state)
            .map_err(|err| err.to_string())?
            .script_pubkey
    };
    let observed = output_script(node, &current.txid.to_string(), current.vout)?;
    let creation = inclusion(node, &current.txid.to_string())?;
    let unspent = match node.call(
        false,
        "gettxout",
        json!([current.txid.to_string(), current.vout, false]),
    ) {
        Ok(Value::Null) => false,
        Ok(_) => true,
        Err(err) => return Err(err),
    };
    let spend_proof = match spend {
        Some(txid) if !unspent => Some(inclusion(node, &txid.to_string())?),
        _ => None,
    };
    let (best, best_block_hash) = tip(node)?;
    let evidence = LineageEvidence {
        seals: vec![SealFact {
            expected_script: expected,
            observed_script: observed,
            creation: creation.clone(),
        }],
        anchor: Some(inclusion(node, &current.txid.to_string())?),
        observation: Some(CurrentSealView {
            unspent,
            spend: spend_proof,
        }),
        o2a_ok: verify(&identity.genesis).is_ok(),
        valid_transition,
        best_height: best,
        required_depth: depth,
    };
    let report = evaluate_lineage(&evidence, rgb);
    let confirmations = u64::from(best.saturating_add(1).saturating_sub(creation.height));
    Ok(StageObservation {
        text: format!(
            "source={}\nbest_height={best}\n{}",
            crate::format::SEAL_SOURCE,
            format_lineage_report(&report)
        ),
        state: report.identity_history_state,
        bitcoin: report.bitcoin,
        unspent,
        confirmations,
        best_height: best,
        best_block_hash,
    })
}

fn successor(prior: &Identity, state: &ResultingState, next: OutPoint) -> Identity {
    Identity {
        entity: prior.entity,
        genesis: prior.genesis.clone(),
        state: state.clone(),
        outpoint: next,
        contract_id: prior.contract_id,
        opout: prior.opout,
        bytes: prior.bytes.clone(),
        rgb_ok: prior.rgb_ok,
    }
}

fn rotated_state(
    previous: &ResultingState,
    genesis: &SignedObject,
    keys: &DemoKeys,
    next_seal: [u8; 36],
) -> Result<ResultingState, String> {
    let entity = entity_id(&genesis.payload);
    let mut state = previous.clone();
    state.sequence = previous.sequence.saturating_add(1);
    state.previous_state = Some(state_id(&entity, &encode_resulting_state(previous)));
    state.previous_seal = Some(previous.next_seal);
    state.next_seal = next_seal;
    state.controllers = vec![ControllerEntry {
        xonly: keys.controller_1.xonly,
        capabilities: previous
            .controllers
            .first()
            .map(|entry| entry.capabilities.clone())
            .unwrap_or_default(),
    }];
    state.controller_bindings = vec![SealBinding {
        authorizing_key_id: key_id(1, keys.controller_1.xonly),
        seal_xonly: keys.seal_controller_1.xonly,
    }];
    Ok(state)
}

fn decode_named(object: &SignedObject, outpoint: &OutPoint) -> Result<ResultingState, String> {
    let canonical = super::canonical_outpoint(outpoint.txid, outpoint.vout);
    state_named_by_signed(&[&object.payload], &canonical).map_err(|err| err.to_string())
}

fn seal_key(state: &ResultingState, keys: &DemoKeys) -> Result<DemoKey, String> {
    let xonly = state
        .controller_bindings
        .iter()
        .map(|binding| binding.seal_xonly)
        .min()
        .ok_or("signed state has no controller binding")?;
    [keys.seal_controller_0, keys.seal_controller_1]
        .into_iter()
        .find(|key| key.xonly == xonly)
        .ok_or_else(|| "decoded controller binding is not a key of this entity".into())
}

fn replace_non_seal(node: &super::chain::Node) -> Result<String, String> {
    let address = node
        .call(true, "getnewaddress", json!([]))?
        .as_str()
        .ok_or("address")?
        .to_string();
    let first = node.call(
        true,
        "sendtoaddress",
        json!([address, "0.001", "", "", false, true]),
    )?;
    let txid1 = first.as_str().unwrap_or("").to_string();
    let bumped = node.call(true, "bumpfee", json!([txid1]))?;
    let txid2 = bumped
        .get("txid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    node.mine(1)?;
    Ok(format!(
        "txid1 {txid1} txid2 {txid2} txid_changed {}",
        !txid1.is_empty() && txid1 != txid2
    ))
}

fn cpfp_change(node: &super::chain::Node, parent: &Funded) -> Result<String, String> {
    let tx: Transaction = deserialize(&hex::decode(&parent.hex).map_err(|err| err.to_string())?)
        .map_err(|err| err.to_string())?;
    let seal_vout = parent.point.vout as usize;
    let change = tx
        .output
        .iter()
        .enumerate()
        .find(|(index, _)| *index != seal_vout)
        .ok_or("funding tx has no change output")?;
    let change_value = change.1.value.to_sat();
    if change_value <= 20_000 {
        return Err("change output is too small to CPFP".into());
    }
    let destination = node
        .call(true, "getnewaddress", json!([]))?
        .as_str()
        .ok_or("cpfp address")?
        .to_string();
    let pay = change_value - 20_000;
    let raw = node.call(
        true,
        "createrawtransaction",
        json!([
            [{ "txid": parent.point.txid.to_string(), "vout": change.0 }],
            { destination: super::btc(pay) }
        ]),
    )?;
    let signed = node.call(true, "signrawtransactionwithwallet", json!([raw]))?;
    if signed.get("complete").and_then(Value::as_bool) != Some(true) {
        return Err(format!("cpfp did not sign {signed}"));
    }
    let hex_tx = signed
        .get("hex")
        .and_then(Value::as_str)
        .ok_or("cpfp hex")?
        .to_string();
    let child = node.call(false, "sendrawtransaction", json!([hex_tx]))?;
    let still = node.call(
        false,
        "getrawtransaction",
        json!([parent.point.txid.to_string(), false]),
    )?;
    let unchanged = still.as_str() == Some(parent.hex.as_str());
    Ok(format!(
        "parent {} child {child} parent_txid_unchanged {unchanged} change_vout {}",
        parent.point.txid, change.0
    ))
}

fn fund_output(
    node: &super::chain::Node,
    script: &ScriptBuf,
    sats: u64,
    replaceable: bool,
) -> Result<Funded, String> {
    let unspent = node.call(true, "listunspent", json!([1]))?;
    let utxo = unspent
        .as_array()
        .and_then(|items| items.first())
        .ok_or("no mature wallet output")?
        .clone();
    let sequence = if replaceable {
        4_294_967_293u32
    } else {
        4_294_967_294u32
    };
    let inputs = json!([{ "txid": utxo["txid"], "vout": utxo["vout"], "sequence": sequence }]);
    let address = Address::from_script(script.as_script(), crate::profile::load()?.bitcoin)
        .map_err(|err| err.to_string())?;
    let raw = node
        .call(
            true,
            "createrawtransaction",
            json!([inputs, [{ address.to_string(): super::btc(sats) }], 0, replaceable]),
        )?
        .as_str()
        .ok_or("createrawtransaction")?
        .to_string();
    let options = json!({ "changePosition": 1, "fee_rate": 5, "replaceable": replaceable });
    let funded = match node.call(true, "fundrawtransaction", json!([raw, options])) {
        Ok(value) => value,
        Err(_) => node.call(
            true,
            "fundrawtransaction",
            json!([raw, { "changePosition": 1, "replaceable": replaceable }]),
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
    let hex_tx = signed
        .get("hex")
        .and_then(Value::as_str)
        .ok_or("signed hex")?
        .to_string();
    let tx: Transaction = deserialize(&hex::decode(&hex_tx).map_err(|err| err.to_string())?)
        .map_err(|err| err.to_string())?;
    let sequences = tx
        .input
        .iter()
        .map(|input| input.sequence.to_consensus_u32())
        .collect::<Vec<_>>();
    let vout = tx
        .output
        .iter()
        .position(|output| {
            output.value.to_sat() == sats && output.script_pubkey.as_bytes() == script.as_bytes()
        })
        .ok_or("funded tx is missing the requested output")? as u32;
    let txid_text = node
        .call(false, "sendrawtransaction", json!([hex_tx]))?
        .as_str()
        .ok_or("funding txid")?
        .to_string();
    let txid = Txid::from_str(&txid_text).map_err(|err| err.to_string())?;
    Ok(Funded {
        point: OutPoint { txid, vout },
        sequences,
        hex: hex_tx,
    })
}

fn reorg_above(node: &super::chain::Node, h0: u32) -> Result<String, String> {
    let before = node.height()?;
    if before <= h0 {
        return Ok(format!("above_h0 false height {before} h0 {h0}"));
    }
    let tip = node
        .call(false, "getblockhash", json!([before]))?
        .as_str()
        .ok_or("tip")?
        .to_string();
    let h0_hash = node
        .call(false, "getblockhash", json!([h0]))?
        .as_str()
        .unwrap_or("")
        .to_string();
    if tip == h0_hash {
        return Ok("above_h0 false tip is H0".into());
    }
    node.call(false, "invalidateblock", json!([tip]))?;
    let dropped = node.height()?;
    node.call(false, "reconsiderblock", json!([tip]))?;
    let restored = node.height()?;
    super::chain::wait_electrs(restored)?;
    Ok(format!(
        "h0 {h0} h0_hash {h0_hash} before {before} dropped {dropped} restored_height {restored} above_h0 {} restored {}",
        dropped > h0 && dropped < before,
        restored == before
    ))
}

fn inclusion(node: &super::chain::Node, txid: &str) -> Result<InclusionProof, String> {
    let tx = node.call(false, "getrawtransaction", json!([txid, true]))?;
    let blockhash = tx
        .get("blockhash")
        .and_then(Value::as_str)
        .ok_or("transaction is not in a block")?
        .to_string();
    let header = node.call(false, "getblockheader", json!([blockhash]))?;
    let height = header
        .get("height")
        .and_then(Value::as_u64)
        .ok_or("header height")? as u32;
    let hex_header = node
        .call(false, "getblockheader", json!([blockhash, false]))?
        .as_str()
        .ok_or("header hex")?
        .trim()
        .to_string();
    let raw = hex::decode(hex_header).map_err(|err| err.to_string())?;
    if raw.len() != 80 {
        return Err(format!("header is {} bytes", raw.len()));
    }
    let mut header_bytes = [0u8; 80];
    header_bytes.copy_from_slice(&raw);
    let block = node.call(false, "getblock", json!([blockhash, 1]))?;
    let listed = block
        .get("tx")
        .and_then(Value::as_array)
        .ok_or("block tx list")?;
    let mut ids = Vec::new();
    let mut index = None;
    for (position, item) in listed.iter().enumerate() {
        let text = item.as_str().ok_or("block txid")?;
        if text.eq_ignore_ascii_case(txid) {
            index = Some(position as u32);
        }
        ids.push(
            Txid::from_str(text)
                .map_err(|err| err.to_string())?
                .to_byte_array(),
        );
    }
    let index = index.ok_or("txid is not in the block")?;
    let siblings = merkle_siblings(&ids, index);
    let proof = InclusionProof {
        txid: ids[index as usize],
        index,
        siblings,
        header: header_bytes,
        height,
    };
    if !inclusion_matches(&proof) {
        return Err(format!("merkle proof does not match the header for {txid}"));
    }
    Ok(proof)
}

fn merkle_siblings(ids: &[[u8; 32]], mut index: u32) -> Vec<[u8; 32]> {
    let mut level = ids.to_vec();
    let mut siblings = Vec::new();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            let last = *level.last().expect("level");
            level.push(last);
        }
        let sibling = (index ^ 1) as usize;
        siblings.push(level[sibling]);
        let mut next = Vec::new();
        for pair in level.chunks(2) {
            next.push(hash256(&pair[0], &pair[1]));
        }
        index /= 2;
        level = next;
    }
    siblings
}

fn hash256(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    use rgbstd::bitcoin::hashes::{sha256d, Hash};
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(left);
    data[32..].copy_from_slice(right);
    sha256d::Hash::hash(&data).to_byte_array()
}

fn output_script(node: &super::chain::Node, txid: &str, vout: u32) -> Result<Vec<u8>, String> {
    let tx = node.call(false, "getrawtransaction", json!([txid, true]))?;
    let hex_script = tx
        .get("vout")
        .and_then(Value::as_array)
        .and_then(|items| items.get(vout as usize))
        .and_then(|output| output.get("scriptPubKey"))
        .and_then(|script| script.get("hex"))
        .and_then(Value::as_str)
        .ok_or("missing output script")?;
    hex::decode(hex_script).map_err(|err| err.to_string())
}

fn core_address(node: &super::chain::Node, state: &ResultingState) -> Result<String, String> {
    let seal = seal_for_state(state).map_err(|err| err.to_string())?;
    let script = ScriptBuf::from_bytes(seal.script_pubkey);
    let local = Address::from_script(script.as_script(), crate::profile::load()?.bitcoin)
        .map_err(|err| err.to_string())?
        .to_string();
    let info = node.call(false, "getdescriptorinfo", json!([core_descriptor(state)]))?;
    let descriptor = info
        .get("descriptor")
        .and_then(Value::as_str)
        .ok_or("descriptor checksum")?;
    let derived = node.call(false, "deriveaddresses", json!([descriptor]))?;
    let core = derived
        .as_array()
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .ok_or("deriveaddresses")?;
    if core != local {
        return Err(format!("core {core} local {local}"));
    }
    Ok(local)
}

pub(crate) fn core_descriptor(state: &ResultingState) -> String {
    let mut recovery = state
        .recovery_bindings
        .iter()
        .map(|binding| binding.seal_xonly)
        .collect::<Vec<_>>();
    recovery.sort();
    let keys = recovery
        .iter()
        .map(hex::encode)
        .collect::<Vec<_>>()
        .join(",");
    let older = format!(
        "and_v(v:multi_a({},{}),older({}))",
        state.recovery.threshold, keys, state.recovery.delay_blocks
    );
    let mut controllers = state
        .controller_bindings
        .iter()
        .map(|binding| binding.seal_xonly)
        .collect::<Vec<_>>();
    controllers.sort();
    let controller = controllers.first().map(hex::encode).unwrap_or_default();
    format!("tr({},{{pk({controller}),{older}}})", hex::encode(NUMS_X))
}

fn write_object(log: &super::Log, name: &str, object: &SignedObject) -> Result<(), String> {
    let body = format!(
        "tag={}\npayload={}\ndigest={}\nsigner={}\nsignature={}\n",
        object.tag,
        hex::encode(&object.payload),
        hex::encode(object.digest),
        hex::encode(object.signer_xonly),
        hex::encode(object.signature)
    );
    log.write(name, body.as_bytes())
}

fn record(
    log: &mut super::Log,
    failed: &mut bool,
    id: &str,
    expected: &str,
    observed: &str,
    ok: bool,
) {
    log.case(id, expected, &super::one_line(observed), ok);
    if !ok {
        *failed = true;
    }
}

fn sequences_final(sequences: &[u32]) -> bool {
    sequences
        .iter()
        .all(|value| *value == 4_294_967_294 || *value == 4_294_967_295)
}

fn field_u32(text: &str, key: &str) -> Option<u32> {
    let token = format!("{key} ");
    let rest = text.split(&token).nth(1)?;
    rest.split_whitespace().next()?.parse().ok()
}

fn use_entity(index: u32) {
    unsafe {
        std::env::set_var("O2A_DEMO_ENTITY", index.to_string());
    }
}

fn require_seed() -> Result<(), String> {
    if std::env::var("O2A_DEMO_SEED_FILE").is_err() {
        return Err("O2A_DEMO_SEED_FILE is required".into());
    }
    Ok(())
}

/// Mainnet plan requires a real seed file. The published-seed preview is
/// regtest and signet only, and only when `O2A_DEMO_UNSAFE_PREVIEW=1`.
fn require_plan_seed(kind: crate::profile::NetworkKind) -> Result<(), String> {
    if std::env::var("O2A_DEMO_SEED_FILE").is_ok() {
        return Ok(());
    }
    let preview = std::env::var("O2A_DEMO_UNSAFE_PREVIEW").as_deref() == Ok("1");
    if kind != crate::profile::NetworkKind::Mainnet && preview {
        eprintln!("preview uses the published unsafe seed; this is not the block-0 identity");
        return Ok(());
    }
    Err("O2A_DEMO_SEED_FILE is required".into())
}

pub(crate) struct FundingObservation {
    pub output_exists: bool,
    pub output_script: Vec<u8>,
    pub sequences: Vec<u32>,
    pub confirmations: u64,
    pub unspent: bool,
    pub inclusion: InclusionProof,
    pub txout_proof: Vec<u8>,
}

pub(crate) trait ReadBackend {
    fn observe_funding(&self, txid: &str, vout: u32) -> Result<FundingObservation, String>;
}

impl ReadBackend for super::chain::Node {
    fn observe_funding(&self, txid: &str, vout: u32) -> Result<FundingObservation, String> {
        let tx = self.call(false, "getrawtransaction", json!([txid, true]))?;
        let confirmations = tx.get("confirmations").and_then(Value::as_u64).unwrap_or(0);
        let sequences = tx
            .get("vin")
            .and_then(Value::as_array)
            .ok_or("vin")?
            .iter()
            .map(|input| input.get("sequence").and_then(Value::as_u64).unwrap_or(0) as u32)
            .collect::<Vec<_>>();
        let outputs = tx.get("vout").and_then(Value::as_array);
        let script_hex = outputs
            .and_then(|items| items.get(vout as usize))
            .and_then(|output| output.get("scriptPubKey"))
            .and_then(|script| script.get("hex"))
            .and_then(Value::as_str);
        let (output_exists, output_script) = match script_hex {
            Some(hex_script) => (
                true,
                hex::decode(hex_script).map_err(|err| err.to_string())?,
            ),
            None => (false, Vec::new()),
        };
        let proof_hex = self
            .call(false, "gettxoutproof", json!([[txid]]))?
            .as_str()
            .unwrap_or("")
            .to_string();
        let txout_proof = hex::decode(proof_hex.trim()).unwrap_or_default();
        let inclusion = inclusion(self, txid)?;
        let utxo = self.call(false, "gettxout", json!([txid, vout, false]))?;
        Ok(FundingObservation {
            output_exists,
            output_script,
            sequences,
            confirmations,
            unspent: !utxo.is_null(),
            inclusion,
            txout_proof,
        })
    }
}

pub(crate) struct GenesisDraft {
    pub signed: SignedObject,
    pub state: ResultingState,
    pub confirmations: u64,
    pub created_at_height: u32,
}

/// Chain checks use the read backend. The signature is produced only after they pass.
pub(crate) fn draft_genesis(
    active: &crate::profile::NetworkProfile,
    outpoint_text: &str,
    view: &dyn ReadBackend,
) -> Result<GenesisDraft, String> {
    let outpoint = parse_outpoint(outpoint_text)?;
    let observed = view.observe_funding(&outpoint.txid.to_string(), outpoint.vout)?;
    if !observed.output_exists {
        return Err("funding output does not exist; genesis stays unsigned".into());
    }
    if !inclusion_matches(&observed.inclusion) {
        return Err(
            "funding merkle proof does not match the block header; genesis stays unsigned".into(),
        );
    }
    if observed.txout_proof.is_empty() {
        return Err(
            "funding transaction has no merkle inclusion proof; genesis stays unsigned".into(),
        );
    }
    if !observed.unspent {
        return Err("funding output is spent; genesis stays unsigned".into());
    }
    if !sequences_final(&observed.sequences) {
        return Err(format!(
            "funding sequences {:?} are replaceable; genesis stays unsigned",
            observed.sequences
        ));
    }
    if observed.confirmations < u64::from(active.required_depth) {
        return Err(format!(
            "depth {}; genesis stays unsigned until {} confirmations",
            observed.confirmations, active.required_depth
        ));
    }
    let canonical = super::canonical_outpoint(outpoint.txid, outpoint.vout);
    let keys = keys_for(active.coin_type);
    let state = genesis_state_from(&keys, canonical);
    let planned = seal_for_state(&state).map_err(|err| err.to_string())?;
    if observed.output_script != planned.script_pubkey {
        return Err(
            "output script does not match the planned seal policy; genesis stays unsigned".into(),
        );
    }
    let signed = genesis_for(active.network_byte, keys.root, &state);
    verify(&signed).map_err(|err| err.to_string())?;
    if signed.payload.get(5..37) != Some(&[0u8; 32]) {
        return Err("genesis signer_entity is not zero".into());
    }
    let decoded =
        state_named_by_signed(&[&signed.payload], &canonical).map_err(|err| err.to_string())?;
    let again = seal_for_state(&decoded).map_err(|err| err.to_string())?;
    if again.script_pubkey != planned.script_pubkey {
        return Err("signed genesis policy does not match the planned script".into());
    }
    Ok(GenesisDraft {
        signed,
        state,
        confirmations: observed.confirmations,
        created_at_height: observed.inclusion.height,
    })
}

#[cfg(test)]
pub(crate) struct CannedFunding {
    pub raw_tx: Vec<u8>,
    pub confirmations: u64,
    pub unspent: bool,
    pub header: [u8; 80],
    pub height: u32,
    pub txout_proof: Vec<u8>,
}

#[cfg(test)]
impl ReadBackend for CannedFunding {
    fn observe_funding(&self, txid: &str, vout: u32) -> Result<FundingObservation, String> {
        let tx: Transaction =
            deserialize(&self.raw_tx).map_err(|err| format!("canned transaction: {err}"))?;
        let parsed = tx.compute_txid();
        if parsed.to_string() != txid {
            return Err(format!("canned txid {parsed} does not match {txid}"));
        }
        let sequences = tx
            .input
            .iter()
            .map(|input| input.sequence.to_consensus_u32())
            .collect::<Vec<_>>();
        let Some(output) = tx.output.get(vout as usize) else {
            return Ok(FundingObservation {
                output_exists: false,
                output_script: Vec::new(),
                sequences,
                confirmations: self.confirmations,
                unspent: self.unspent,
                inclusion: InclusionProof {
                    txid: parsed.to_byte_array(),
                    index: 0,
                    siblings: Vec::new(),
                    header: self.header,
                    height: self.height,
                },
                txout_proof: self.txout_proof.clone(),
            });
        };
        Ok(FundingObservation {
            output_exists: true,
            output_script: output.script_pubkey.as_bytes().to_vec(),
            sequences,
            confirmations: self.confirmations,
            unspent: self.unspent,
            inclusion: InclusionProof {
                txid: parsed.to_byte_array(),
                index: 0,
                siblings: Vec::new(),
                header: self.header,
                height: self.height,
            },
            txout_proof: self.txout_proof.clone(),
        })
    }
}

#[cfg(test)]
pub(crate) fn canned_funding(
    script: &[u8],
    sequence: u32,
    confirmations: u64,
    unspent: bool,
    proof: &[u8],
    matching_header: bool,
) -> (String, CannedFunding) {
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::from_consensus(sequence),
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: ScriptBuf::from_bytes(script.to_vec()),
        }],
    };
    let txid = tx.compute_txid();
    let mut header = [0u8; 80];
    if matching_header {
        header[36..68].copy_from_slice(&txid.to_byte_array());
    }
    let view = CannedFunding {
        raw_tx: serialize(&tx),
        confirmations,
        unspent,
        header,
        height: 100,
        txout_proof: proof.to_vec(),
    };
    (format!("{txid}:0"), view)
}

pub fn genesis(authorize: bool, seal: Option<&str>) -> Result<(), String> {
    let active = crate::profile::load()?;
    crate::profile::begin_cli(&active, crate::profile::Operation::Genesis, authorize)?;
    require_seed()?;
    let outpoint_text = seal
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("genesis --seal <outpoint> is required")?
        .to_string();
    let dir =
        PathBuf::from(std::env::var("RGB011_EVIDENCE").map_err(|_| "RGB011_EVIDENCE is required")?);
    let mut log = super::Log::open(dir)?;
    let node = super::chain::Node::connect_wallet(false)?;
    let draft = draft_genesis(&active, &outpoint_text, &node)?;
    let outpoint = parse_outpoint(&outpoint_text)?;
    let prepared = super::schema::identity_schema();
    let issued = super::schema::issue_on(
        &prepared,
        super::chain::chain_net()?,
        draft.signed.digest,
        outpoint,
        BLINDING,
        super::schema::TS_ISSUE,
    )?;
    let bytes = super::consignment_bytes(&*issued)?;
    let report = super::dual_validate::<false>(&bytes, &prepared.types)?;
    if !report.starts_with("Consignment is valid") {
        return Err(format!(
            "genesis did not validate: {}",
            super::one_line(&report)
        ));
    }
    write_object(&log, "genesis.o2a", &draft.signed)?;
    log.write("genesis.strict", &bytes)?;
    let entity = entity_id(&draft.signed.payload);
    let public = format!(
        "entity_id={}\nstate_id={}\nseal={outpoint_text}\nconfirmations={}\nnetwork={}\ncreated_at_height={}\n",
        hex::encode(entity),
        hex::encode(state_id(&entity, &encode_resulting_state(&draft.state))),
        draft.confirmations,
        active.kind.label(),
        draft.created_at_height
    );
    log.write("public.txt", public.as_bytes())?;
    log.line(&format!("entity_id {}", hex::encode(entity)));
    log.line("seal_unspent true");
    log.line("no transition was signed");
    println!("entity_id={}", hex::encode(entity));
    println!("confirmations={}", draft.confirmations);
    Ok(())
}

pub fn claim(authorize: bool) -> Result<(), String> {
    let active = crate::profile::load()?;
    crate::profile::begin_cli(&active, crate::profile::Operation::OfficialName, authorize)?;
    require_seed()?;
    let dir =
        PathBuf::from(std::env::var("RGB011_EVIDENCE").map_err(|_| "RGB011_EVIDENCE is required")?);
    let genesis = read_object(&dir.join("genesis.o2a"))?;
    let entity = entity_id(&genesis.payload);
    let state = decode_identity_state(&genesis.payload).map_err(|err| err.to_string())?;
    let name = std::env::var("O2A_REHEARSAL_NAME").unwrap_or_else(|_| "Rehearsal Name".into());
    let claim = official_name_claim(
        active.network_byte,
        entity,
        state_id(&entity, &encode_resulting_state(&state)),
        keys_for(active.coin_type).controller_0,
        &name,
        official_name_nonce(&entity, &name),
    )
    .map_err(|err| err.to_string())?;
    verify(&claim).map_err(|err| err.to_string())?;
    let log = super::Log::open(dir)?;
    write_object(&log, "claim.o2a", &claim)?;
    println!("name_claim={name}");
    println!("entity_id={}", hex::encode(entity));
    Ok(())
}

pub fn stage_verify(authorize: bool, json_out: bool) -> Result<(), String> {
    let active = crate::profile::load()?;
    crate::profile::begin_cli(&active, crate::profile::Operation::Verify, authorize)?;
    if json_out {
        let _ = crate::format::verifier_id()?;
    }
    let dir =
        PathBuf::from(std::env::var("RGB011_EVIDENCE").map_err(|_| "RGB011_EVIDENCE is required")?);
    let genesis = read_object(&dir.join("genesis.o2a"))?;
    verify(&genesis).map_err(|err| err.to_string())?;
    let entity = entity_id(&genesis.payload);
    let state = decode_identity_state(&genesis.payload).map_err(|err| err.to_string())?;
    // The outpoint is the next seal committed in the signed genesis.
    let outpoint = crate::format::outpoint_from_next_seal(&state.next_seal);
    let canonical = super::canonical_outpoint(outpoint.txid, outpoint.vout);
    let decoded =
        state_named_by_signed(&[&genesis.payload], &canonical).map_err(|err| err.to_string())?;
    if decoded != state {
        return Err("verifier state does not match the signed genesis".into());
    }
    let node = super::chain::Node::connect_wallet(false)?;
    let bytes = std::fs::read(dir.join("genesis.strict")).map_err(|err| err.to_string())?;
    let prepared = super::schema::identity_schema();
    let report = super::dual_validate::<false>(&bytes, &prepared.types)?;
    let rgb_status = super::one_line(&report);
    let identity = Identity {
        entity,
        genesis: genesis.clone(),
        state: state.clone(),
        outpoint,
        contract_id: rgbstd::ContractId::copy_from_slice([0u8; 32])
            .map_err(|err| err.to_string())?,
        opout: Opout::new(
            rgbstd::OpId::copy_from_slice([0u8; 32]).map_err(|err| err.to_string())?,
            super::schema::OS_IDENTITY,
            0,
        ),
        bytes,
        rgb_ok: report.starts_with("Consignment is valid"),
    };
    let expected_seal = seal_for_state(&state).map_err(|err| err.to_string())?;
    let spend = spending_txid(&node, outpoint, &expected_seal.script_pubkey)?;
    let observed = o2a_report_with(
        &node,
        &identity,
        outpoint,
        spend,
        false,
        &rgb_status,
        active.required_depth,
    )?;
    let sid = state_id(&entity, &encode_resulting_state(&state));
    let mut reasons = Vec::new();
    if !identity.rgb_ok {
        reasons.push(rgb_status.clone());
    }
    if observed.state != "CURRENT" {
        reasons.push(observed.bitcoin.to_string());
    }
    let claim_path = dir.join("claim.o2a");
    let (claim_valid, official_name, claim_failure) = if claim_path.exists() {
        let claim = read_object(&claim_path)?;
        let controller = state
            .controllers
            .first()
            .ok_or("genesis has no controller")?;
        let kid = key_id(1, controller.xonly);
        let authorization = ClaimAuthorization {
            entity: &entity,
            state: Some(&sid),
            key_id: &kid,
            public_key: &controller.xonly,
            key_role: 1,
            capabilities: &controller.capabilities,
        };
        match evaluate_name_claim(
            &claim.payload,
            claim.signature,
            claim.signer_xonly,
            active.network_byte,
            &authorization,
        ) {
            Ok(()) => match official_name_of(&claim.payload) {
                Ok(name) => (true, Some(name), None),
                Err(err) => {
                    let err = err.to_string();
                    reasons.push(err.clone());
                    (false, None, Some(err))
                }
            },
            Err(err) => {
                let err = err.to_string();
                reasons.push(err.clone());
                (false, None, Some(err))
            }
        }
    } else {
        reasons.push("official_name claim is absent".into());
        (false, None, None)
    };
    if json_out {
        let facts = crate::format::VerifyFacts {
            network: active.kind.label().to_string(),
            verifier_id: crate::format::verifier_id()?,
            entity_id: hex::encode(entity),
            state_id: hex::encode(sid),
            identity_history_state: observed.state.to_string(),
            seal_outpoint: crate::format::seal_text(&state.next_seal),
            confirmations: observed.confirmations,
            required_depth: active.required_depth,
            unspent: observed.unspent,
            source: crate::format::SEAL_SOURCE.to_string(),
            best_block_hash: observed.best_block_hash.clone(),
            height: observed.best_height,
            rgb_status,
            genesis_valid: true,
            claim_valid,
            official_name,
            reasons,
        };
        let document = crate::format::verification_document(&facts)?;
        print!("{document}");
    } else {
        println!("entity_id={}", hex::encode(entity));
        println!("{}", super::one_line(&report));
        println!("{}", observed.text);
        if claim_valid {
            println!("name_claim=valid");
        }
    }
    if let Some(err) = claim_failure {
        return Err(err);
    }
    if observed.state != "CURRENT" || !identity.rgb_ok {
        return Err("verifier did not report CURRENT".into());
    }
    Ok(())
}

fn parse_outpoint(text: &str) -> Result<OutPoint, String> {
    let (txid, vout) = text.trim().split_once(':').ok_or("outpoint")?;
    Ok(OutPoint {
        txid: Txid::from_str(txid).map_err(|err| err.to_string())?,
        vout: vout
            .parse()
            .map_err(|err: std::num::ParseIntError| err.to_string())?,
    })
}

fn read_object(path: &Path) -> Result<SignedObject, String> {
    let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    parse_object(&text)
}

pub(crate) fn parse_object(text: &str) -> Result<SignedObject, String> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.to_string(), value.to_string());
        }
    }
    let tag = match fields.get("tag").map(String::as_str) {
        Some("O2A/v0.1/entity-genesis") => "O2A/v0.1/entity-genesis",
        Some("O2A/v0.1/identity-transition") => "O2A/v0.1/identity-transition",
        Some("O2A/v0.1/recovery") => "O2A/v0.1/recovery",
        Some("O2A/v0.1/claim") => "O2A/v0.1/claim",
        Some(other) => return Err(format!("unknown tag {other}")),
        None => return Err("missing tag".into()),
    };
    let payload =
        hex::decode(fields.get("payload").ok_or("payload")?).map_err(|err| err.to_string())?;
    let digest =
        hex::decode(fields.get("digest").ok_or("digest")?).map_err(|err| err.to_string())?;
    let signer =
        hex::decode(fields.get("signer").ok_or("signer")?).map_err(|err| err.to_string())?;
    let signature =
        hex::decode(fields.get("signature").ok_or("signature")?).map_err(|err| err.to_string())?;
    Ok(SignedObject {
        tag,
        payload,
        digest: digest.try_into().map_err(|_| "digest length")?,
        signer_xonly: signer.try_into().map_err(|_| "signer length")?,
        signature: signature.try_into().map_err(|_| "signature length")?,
    })
}
