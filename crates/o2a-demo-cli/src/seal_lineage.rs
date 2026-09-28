use std::path::Path;
use std::str::FromStr;

use std::fs;

use amplify::confinement::SmallOrdSet;
use amplify::ByteArray;
use anyhow::{bail, Context, Result};
use bpstd::psbt::PsbtConstructor;
use bpstd::signers::TestnetSigner;
use bpstd::{Derive, Descriptor, NormalIndex, Outpoint, ScriptPubkey, Terminal, Witness};
use o2a_demo_core::{
    controller_rotation_with, demo_delay_blocks, demo_entity_index, demo_genesis_state, demo_keys,
    demo_network, demo_rotation_state, demo_threshold, encode_resulting_state, entity_id,
    genesis_with, key_id, official_name_claim, official_name_nonce, recovery_authorizations,
    seal_bindings_valid, seal_for_state, state_id, ResultingState,
};
use o2a_demo_rgb::{
    controller_rotation, demo_issuer, genesis_params, recover, GenesisInput, RotationInput,
};
use o2a_demo_seal::{
    finish_spend, gather_report, open_spend, prepare_state, script_path_tx, Electrum, SealRecord,
    SealStore, SpendRequest, SpendStyle, TrackedSeal,
};
use rgb::popls::bp::{Prefab, PrefabBundle, WalletProvider};
use rgb::{CellAddr, RgbSealDef};
use rgbp::descriptors::RgbDescr;
use rgbp::resolvers::ElectrumResolver;
use rgbp::{FileHolder, Owner};
use rgpsbt::RgbPsbt;

use crate::{canonical_outpoint, external_seal, runtime, write_object};

fn ensure_wallet(data_dir: &Path, electrum: &str) -> Result<()> {
    if data_dir.join("wallet").exists() {
        return Ok(());
    }
    std::fs::create_dir_all(data_dir)?;
    let holder = FileHolder::create(data_dir.join("wallet"), crate::descriptor())?;
    let resolver = ElectrumResolver::new(electrum)?;
    let mut owner = Owner::with_components(crate::demo_bp_network(), holder, resolver);
    println!("fee_address={}", owner.next_address());
    Ok(())
}

pub fn plan() -> Result<()> {
    let keys = demo_keys();
    let prepared = prepare_state("genesis").map_err(anyhow::Error::msg)?;
    print!("{}", prepared.text);
    println!("entity_index={entity}", entity = demo_entity_index());
    println!("network={}", demo_network());
    println!("delay_blocks={}", demo_delay_blocks());
    println!("threshold={}", demo_threshold());
    println!("root_xonly={}", hex::encode(keys.root.xonly));
    println!("controller_xonly={}", hex::encode(keys.controller_0.xonly));
    println!("recovery_0_xonly={}", hex::encode(keys.recovery_0.xonly));
    println!("recovery_1_xonly={}", hex::encode(keys.recovery_1.xonly));
    println!("recovery_2_xonly={}", hex::encode(keys.recovery_2.xonly));
    println!("seal_0_xonly={}", hex::encode(keys.seal_controller_0.xonly));
    println!("root_key_id={}", hex::encode(key_id(0, keys.root.xonly)));
    println!(
        "controller_key_id={}",
        hex::encode(key_id(1, keys.controller_0.xonly))
    );
    println!(
        "recovery_0_key_id={}",
        hex::encode(key_id(2, keys.recovery_0.xonly))
    );
    println!(
        "recovery_1_key_id={}",
        hex::encode(key_id(2, keys.recovery_1.xonly))
    );
    println!(
        "recovery_2_key_id={}",
        hex::encode(key_id(2, keys.recovery_2.xonly))
    );
    Ok(())
}

pub fn sign_genesis(data_dir: &Path, outpoint: &str) -> Result<()> {
    let parsed = Outpoint::from_str(outpoint)?;
    let state = demo_genesis_state(canonical_outpoint(parsed));
    let state_bytes = encode_resulting_state(&state);
    let object = genesis_with(demo_keys().root, &state);
    o2a_demo_core::verify(&object).map_err(anyhow::Error::msg)?;
    if object.payload.get(5..37) != Some(&[0u8; 32]) {
        bail!("genesis signer_entity is not zero");
    }
    std::fs::create_dir_all(data_dir)?;
    write_object(&data_dir.join("genesis.o2a"), &object)?;
    let id = entity_id(&object.payload);
    println!("entity_id={}", hex::encode(id));
    println!("state_id={}", hex::encode(state_id(&id, &state_bytes)));
    println!("signer_entity_zero=true");
    println!("network={}", demo_network());
    Ok(())
}

fn resulting_state_of_genesis(payload: &[u8]) -> Result<&[u8]> {
    if payload.get(37) != Some(&0) || payload.len() < 107 {
        bail!("genesis payload does not have an absent authorizing state");
    }
    Ok(&payload[107..])
}

pub fn sign_claim(data_dir: &Path, name: &str) -> Result<()> {
    let genesis = read_signed(&data_dir.join("genesis.o2a"))?;
    let id = entity_id(&genesis.payload);
    let state_bytes = resulting_state_of_genesis(&genesis.payload)?;
    let sid = state_id(&id, state_bytes);
    let claim = official_name_claim(
        demo_network(),
        id,
        sid,
        demo_keys().controller_0,
        name,
        official_name_nonce(&id, name),
    )
    .map_err(anyhow::Error::msg)?;
    write_object(&data_dir.join("claim.o2a"), &claim)?;
    println!("entity_id={}", hex::encode(id));
    println!("state_id={}", hex::encode(sid));
    println!("predicate=official_name");
    println!("name={name}");
    println!("signer={}", hex::encode(claim.signer_xonly));
    Ok(())
}

pub fn verify_claim(data_dir: &Path) -> Result<()> {
    let genesis = read_signed(&data_dir.join("genesis.o2a"))?;
    let claim = read_signed(&data_dir.join("claim.o2a"))?;
    o2a_demo_core::verify(&genesis).map_err(anyhow::Error::msg)?;
    o2a_demo_core::verify(&claim).map_err(anyhow::Error::msg)?;
    if claim.tag != "O2A/v0.1/claim" {
        bail!("claim tag is {}", claim.tag);
    }
    let id = entity_id(&genesis.payload);
    let sid = state_id(&id, resulting_state_of_genesis(&genesis.payload)?);
    if claim.payload.get(37) != Some(&1) || claim.payload.len() < 141 {
        bail!("claim header is not the frozen official_name shape");
    }
    if claim.payload[38..70] != sid {
        bail!("claim authorizing state is not the genesis state id");
    }
    if claim.payload[5..37] != id {
        bail!("claim signer entity is not the genesis EntityID");
    }
    if claim.payload[105..137] != id {
        bail!("claim subject is not the genesis EntityID");
    }
    let predicate_len = u32::from_le_bytes(claim.payload[137..141].try_into().expect("len"));
    let predicate_end = 141 + predicate_len as usize;
    if claim.payload.get(141..predicate_end) != Some(b"official_name") {
        bail!("claim predicate is not official_name");
    }
    println!("claim_signature=valid");
    println!("entity_id={}", hex::encode(id));
    println!("state_id={}", hex::encode(sid));
    println!("predicate=official_name");
    Ok(())
}

pub fn prepare(data_dir: &Path, stage: &str) -> Result<()> {
    let prepared = prepare_state(stage).map_err(anyhow::Error::msg)?;
    print!("{}", prepared.text);
    if !data_dir.join("wallet").exists() {
        std::fs::create_dir_all(data_dir)?;
        let holder = FileHolder::create(data_dir.join("wallet"), crate::descriptor())?;
        let resolver = ElectrumResolver::new("tcp://electrs:50001")?;
        let mut owner = Owner::with_components(crate::demo_bp_network(), holder, resolver);
        println!("fee_address={}", owner.next_address());
    }
    Ok(())
}

pub fn record(
    data_dir: &Path,
    name: &str,
    outpoint: &str,
    txid: &str,
    height: u32,
    value: u64,
    stage: &str,
) -> Result<()> {
    let prepared = prepare_state(stage).map_err(anyhow::Error::msg)?;
    SealStore::open(data_dir)?.write(&SealRecord {
        name: name.to_owned(),
        outpoint: outpoint.to_owned(),
        policy_hex: hex::encode(prepared.policy),
        script_pubkey: hex::encode(prepared.script_pubkey),
        paths: prepared.paths,
        stage: stage.to_owned(),
        funding_txid: txid.to_owned(),
        confirmation_height: height,
        value_sats: value,
    })?;
    println!("recorded={name}");
    println!("outpoint={outpoint}");
    Ok(())
}

pub fn issue(data_dir: &Path, electrum: &str, name: &str) -> Result<()> {
    let record = SealStore::open(data_dir)?.read(name)?;
    let outpoint = Outpoint::from_str(&record.outpoint)?;
    ensure_wallet(data_dir, electrum)?;
    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let seen = runtime.wallet.utxos().any(|utxo| utxo == outpoint);
    println!("wallet_contains_seal={seen}");
    if seen {
        bail!("RGB wallet selected the seal");
    }
    let signed_path = data_dir.join("genesis.o2a");
    let object = if signed_path.exists() {
        let object = read_signed(&signed_path)?;
        o2a_demo_core::verify(&object).map_err(anyhow::Error::msg)?;
        object
    } else {
        let object = genesis_with(
            demo_keys().root,
            &demo_genesis_state(canonical_outpoint(outpoint)),
        );
        o2a_demo_core::verify(&object).map_err(anyhow::Error::msg)?;
        write_object(&signed_path, &object)?;
        object
    };
    let keys = demo_keys();
    runtime.contracts.import_issuer(demo_issuer())?;
    let contract_id = runtime.issue(genesis_params(GenesisInput {
        root_xonly: keys.root.xonly,
        entity_id: o2a_demo_core::entity_id(&object.payload),
        controller_xonly: keys.controller_0.xonly,
        policy_hash: o2a_demo_core::recovery_policy_hash(&o2a_demo_core::demo_recovery_policy()),
        state_commitment: object.digest,
        seal: outpoint,
    }))?;
    let state = runtime.contracts.contract_state(contract_id);
    let owned = state
        .owned
        .values()
        .flat_map(|items| items.iter())
        .next()
        .context("issued contract has no owned state")?;
    if !signed_path.exists() {
        write_object(&signed_path, &object)?;
    }
    let consignment = data_dir.join("genesis.rgb");
    runtime
        .contracts
        .export_to_file(&consignment, contract_id)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    std::fs::write(
        data_dir.join("lineage.txt"),
        format!(
            "contract_id={contract_id}\ncell={}\nseal={}\ndigest={}\nname={name}\ncurrent={name}\nanchor_txid={}\nconsignment={}\nsequence=0\n",
            owned.addr,
            record.outpoint,
            hex::encode(object.digest),
            record.funding_txid,
            consignment.display()
        ),
    )?;
    println!(
        "entity_id={}",
        hex::encode(o2a_demo_core::entity_id(&object.payload))
    );
    println!("entity_index={}", o2a_demo_core::demo_entity_index());
    println!("contract_id={contract_id}");
    println!("consignment={}", consignment.display());
    println!("cell={}", owned.addr);
    println!("seal={}", record.outpoint);
    println!("o2a_genesis=valid");
    Ok(())
}

pub fn transition(
    data_dir: &Path,
    electrum: &str,
    from_name: &str,
    to_outpoint: &str,
    fee_outpoint: &str,
    fee_value: u64,
    fee_script_hex: &str,
    kind: &str,
    broadcast: bool,
) -> Result<()> {
    let store = SealStore::open(data_dir)?;
    let from = store.read(from_name)?;
    let stored = crate::read_fields(&data_dir.join("lineage.txt"))?;
    let contract_id = rgb::ContractId::from_str(crate::field(&stored, "contract_id")?)?;
    let previous_cell = CellAddr::from_str(crate::field(&stored, "cell")?)?;
    let previous = Outpoint::from_str(&from.outpoint)?;
    let next = Outpoint::from_str(to_outpoint)?;
    let fee = Outpoint::from_str(fee_outpoint)?;
    let fee_script = ScriptPubkey::from_checked(hex::decode(fee_script_hex)?);
    let keys = demo_keys();
    let prior = previous_cell.opid.to_byte_array();
    let sequence = crate::field(&stored, "sequence")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    let state = demo_rotation_state(
        sequence,
        prior,
        canonical_outpoint(previous),
        canonical_outpoint(next),
    );
    let spent = state_for_stage(&from.stage)?;
    let (signer, rgb_call_recover) = if kind == "recover" {
        (keys.recovery_0, true)
    } else if kind == "rotate-1" {
        (keys.controller_1, false)
    } else {
        (keys.controller_0, false)
    };
    let history = history_entity(data_dir)?;
    let object = if rgb_call_recover {
        let auths = recovery_authorizations(
            history,
            prior,
            &o2a_demo_core::demo_recovery_policy(),
            from.confirmation_height + 10,
            &state,
            &[keys.recovery_0, keys.recovery_2],
        )
        .map_err(anyhow::Error::msg)?;
        auths[0].clone()
    } else {
        controller_rotation_with(signer, history, prior, &state)
    };
    if !rgb_call_recover {
        o2a_demo_core::verify_controller_rotation(&object).map_err(anyhow::Error::msg)?;
    }
    let mut chain = runtime(data_dir, electrum)?;
    chain
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let next_seal = external_seal(next);
    let rgb_input = RotationInput {
        previous_cell,
        controller_xonly: if rgb_call_recover {
            keys.controller_1.xonly
        } else {
            signer.xonly
        },
        state_commitment: object.digest,
        next_seal,
    };
    let call = if rgb_call_recover {
        recover(rgb_input)
    } else {
        controller_rotation(rgb_input)
    };
    let operation = chain
        .contracts
        .contract_call(contract_id, call.params, call.seals)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let defined_cell = CellAddr::new(operation.opid(), 0);
    let change_script = chain.wallet.next_address().script_pubkey();
    let style = if rgb_call_recover {
        let mut slots = [
            keys.seal_recovery_0,
            keys.seal_recovery_1,
            keys.seal_recovery_2,
        ];
        slots.sort_by_key(|key| key.xonly);
        SpendStyle::Recovery {
            slots: slots
                .into_iter()
                .map(|key| {
                    if key.xonly == keys.seal_recovery_0.xonly
                        || key.xonly == keys.seal_recovery_2.xonly
                    {
                        Some(key)
                    } else {
                        None
                    }
                })
                .collect(),
        }
    } else {
        let seal_key = if kind == "rotate-1" {
            keys.seal_controller_1
        } else {
            keys.seal_controller_0
        };
        SpendStyle::Controller { signer: seal_key }
    };
    let mut open = open_spend(
        &SpendRequest {
            seal: previous,
            seal_value: from.value_sats,
            fee,
            fee_value,
            fee_script: fee_script.clone(),
            change_script,
            fee_sats: 1_000,
            state: spent,
        },
        style,
    )?;
    open.psbt
        .output_mut(0)
        .context("host")?
        .set_opret_host()
        .map_err(|_| anyhow::anyhow!("opret host"))?;
    open.psbt.complete_construction();
    let prefab = Prefab {
        closes: SmallOrdSet::try_from_iter([previous])?,
        defines: SmallOrdSet::new(),
        operation,
    };
    let bundle = PrefabBundle::new([prefab])?;
    open.psbt.rgb_fill_csv(&bundle)?;
    open.psbt = chain
        .complete(open.psbt, &bundle)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    fill_fee_tap(&mut open.psbt, chain.wallet.descriptor(), &fee_script)?;
    let fee_signer = TestnetSigner::new(crate::payment_account());
    match open.psbt.sign(&fee_signer) {
        Ok(count) => println!("fee_signatures={count}"),
        Err(error) => println!("fee_sign_error={error}"),
    }
    let finalized = open.psbt.finalize(chain.wallet.descriptor());
    println!("descriptor_finalized_inputs={finalized}");
    if !open.psbt.input(1).context("fee")?.is_finalized() {
        let signature = open
            .psbt
            .input(1)
            .context("fee")?
            .tap_key_sig
            .clone()
            .context("fee input has no taproot key signature")?;
        let witness = Witness::from_consensus_stack(vec![signature.to_vec()]);
        let input = open.psbt.input_mut(1).context("fee")?;
        input.final_script_sig = Some(bpstd::SigScript::empty());
        input.final_witness = Some(witness);
        println!("fee_witness_from_tap_key_sig=set");
    }
    let raw = finish_spend(open)?;
    println!("raw_tx={raw}");
    let path = data_dir.join(format!("{kind}.raw"));
    std::fs::write(&path, &raw)?;
    write_object(&data_dir.join(format!("{kind}.o2a")), &object)?;
    println!("history_entity={}", hex::encode(history));
    println!("defined_cell={defined_cell}");
    println!("o2a_digest={}", hex::encode(object.digest));
    println!("next_seal={to_outpoint}");
    if broadcast {
        let broadcast_text = Electrum::connect(electrum)?
            .call("blockchain.transaction.broadcast", &format!("[\"{raw}\"]"))?;
        let anchor = result_string(&broadcast_text)?;
        println!("broadcast={broadcast_text}");
        println!("anchor_txid={anchor}");
        let consignment = data_dir.join(format!("{kind}.rgb"));
        if consignment.exists() {
            bail!("consignment {} already exists", consignment.display());
        }
        chain.contracts.consign_to_file(
            &consignment,
            contract_id,
            [external_seal(next).auth_token()],
        )?;
        let next_name = store
            .list()?
            .into_iter()
            .find(|item| item.outpoint == to_outpoint)
            .context("next seal is not recorded")?
            .name;
        fs::write(
            data_dir.join("lineage.txt"),
            format!(
                "contract_id={contract_id}\ncell={defined_cell}\nseal={to_outpoint}\ndigest={}\nname={next_name}\ncurrent={next_name}\nanchor_txid={anchor}\nconsignment={}\nsequence={sequence}\n",
                hex::encode(object.digest),
                consignment.display()
            ),
        )?;
        println!("current={next_name}");
        println!("consignment={}", consignment.display());
    }
    let _ = (signer, state);
    Ok(())
}

pub fn close_plain(data_dir: &Path, name: &str, destination_hex: &str) -> Result<()> {
    let record = SealStore::open(data_dir)?.read(name)?;
    let keys = demo_keys();
    let state = state_for_stage(&record.stage)?;
    let destination = ScriptPubkey::from_checked(hex::decode(destination_hex)?);
    let raw = o2a_demo_seal::close_plain_tx(
        SpendRequest {
            seal: Outpoint::from_str(&record.outpoint)?,
            seal_value: record.value_sats,
            fee: Outpoint::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000:0",
            )?,
            fee_value: 0,
            fee_script: destination.clone(),
            change_script: destination.clone(),
            fee_sats: 1_000,
            state,
        },
        if record.stage == "genesis" {
            keys.seal_controller_0
        } else {
            keys.seal_controller_1
        },
        destination,
    )?;
    println!("raw_tx={raw}");
    println!("close_plain=no_rgb_commitment");
    Ok(())
}

pub fn verify(
    electrum: &str,
    data_dir: &Path,
    name: &str,
    validator: Option<&Path>,
    skip_observation: bool,
) -> Result<()> {
    let records = SealStore::open(data_dir)?.list()?;
    let current = records
        .iter()
        .position(|record| record.name == name)
        .context("seal record")?;
    let mut tracked = Vec::new();
    for record in &records {
        let expected = seal_for_state(&state_for_stage(&record.stage)?)
            .map_err(anyhow::Error::msg)?
            .script_pubkey;
        tracked.push(TrackedSeal {
            expected_script: expected,
            outpoint: record.outpoint.clone(),
            funding_txid: record.funding_txid.clone(),
        });
    }
    let lineage = data_dir.join("lineage.txt");
    let stored = if lineage.exists() {
        crate::read_fields(&lineage)?
    } else {
        std::collections::BTreeMap::new()
    };
    let anchor = stored
        .get("anchor_txid")
        .cloned()
        .unwrap_or_else(|| records[current].funding_txid.clone());
    let rgb = match validator {
        Some(dir) => import_consignment(dir, &stored, &records[current].outpoint)?,
        None => "consignment checked separately",
    };
    let mut client = Electrum::connect(electrum)?;
    let report = gather_report(
        &mut client,
        &tracked,
        current,
        &anchor,
        objects_ok(data_dir),
        rgb,
        demo_depth(),
        skip_observation,
    )?;
    if let Ok(genesis) = read_signed(&data_dir.join("genesis.o2a")) {
        println!(
            "entity_id={}",
            hex::encode(o2a_demo_core::entity_id(&genesis.payload))
        );
    }
    println!("entity_index={}", o2a_demo_core::demo_entity_index());
    println!("{report}");
    Ok(())
}

pub fn fee_address(data_dir: &Path) -> Result<()> {
    let holder = FileHolder::load(data_dir.join("wallet"))?;
    let resolver = rgbp::resolvers::ElectrumResolver::new("tcp://electrs:50001")?;
    let mut owner = Owner::with_components(crate::demo_bp_network(), holder, resolver);
    println!("fee_address={}", owner.next_address());
    Ok(())
}

pub fn export_consignment(data_dir: &Path, electrum: &str, path: &Path) -> Result<()> {
    if path.exists() {
        bail!("consignment {} already exists", path.display());
    }
    let stored = crate::read_fields(&data_dir.join("lineage.txt"))?;
    let contract_id = rgb::ContractId::from_str(crate::field(&stored, "contract_id")?)?;
    let runtime = runtime(data_dir, electrum)?;
    runtime
        .contracts
        .export_to_file(path, contract_id)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let mut text = fs::read_to_string(data_dir.join("lineage.txt"))?;
    if !text.contains("consignment=") {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("consignment={}\n", path.display()));
    }
    if !text.contains("anchor_txid=") {
        text.push_str(&format!(
            "anchor_txid={}\ncurrent={}\nsequence=0\n",
            crate::field(&stored, "seal")?
                .split_once(':')
                .context("seal")?
                .0,
            stored.get("name").map(String::as_str).unwrap_or("A")
        ));
    }
    fs::write(data_dir.join("lineage.txt"), text)?;
    println!("consignment={}", path.display());
    Ok(())
}

pub fn presign_recovery(data_dir: &Path, name: &str, destination_hex: &str) -> Result<()> {
    let record = SealStore::open(data_dir)?.read(name)?;
    let destination = ScriptPubkey::from_checked(hex::decode(destination_hex)?);
    let raw = script_path_tx(
        &SpendRequest {
            seal: Outpoint::from_str(&record.outpoint)?,
            seal_value: record.value_sats,
            fee: Outpoint::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000:0",
            )?,
            fee_value: 0,
            fee_script: destination.clone(),
            change_script: destination.clone(),
            fee_sats: 1_000,
            state: state_for_stage(&record.stage)?,
        },
        SpendStyle::RecoveryClose {
            slots: recovery_slots(),
            destination,
        },
    )?;
    println!("raw_tx={raw}");
    println!("presign_recovery=bip68");
    Ok(())
}

fn state_for_stage(stage: &str) -> Result<ResultingState> {
    match stage {
        "genesis" => Ok(demo_genesis_state([0; 36])),
        "rotate" | "recover" => Ok(demo_rotation_state(1, [0; 32], [0; 36], [0; 36])),
        other => bail!("unknown seal stage {other}"),
    }
}

fn recovery_slots() -> Vec<Option<o2a_demo_core::DemoKey>> {
    let keys = demo_keys();
    let mut slots = [
        keys.seal_recovery_0,
        keys.seal_recovery_1,
        keys.seal_recovery_2,
    ];
    slots.sort_by_key(|key| key.xonly);
    slots
        .into_iter()
        .map(|key| {
            if key.xonly == keys.seal_recovery_0.xonly || key.xonly == keys.seal_recovery_2.xonly {
                Some(key)
            } else {
                None
            }
        })
        .collect()
}

fn result_string(text: &str) -> Result<String> {
    let pattern = "\"result\"";
    let start = text.find(pattern).context("missing result")?;
    let rest = &text[start + pattern.len()..];
    let quote = rest.find('"').context("result string")? + 1;
    let end = rest[quote..].find('"').context("result end")? + quote;
    Ok(rest[quote..end].to_owned())
}

fn objects_ok(data_dir: &Path) -> bool {
    let mut found = false;
    let Ok(entries) = fs::read_dir(data_dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".o2a") {
            continue;
        }
        found = true;
        let Ok(object) = read_signed(&entry.path()) else {
            return false;
        };
        if o2a_demo_core::verify(&object).is_err() {
            return false;
        }
    }
    found
}

fn demo_depth() -> u32 {
    std::env::var("O2A_DEMO_DEPTH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

pub fn show_fork(outpoint: &str) -> Result<()> {
    let parsed = Outpoint::from_str(outpoint)?;
    let canonical = canonical_outpoint(parsed);
    let legit = genesis_with(demo_keys().root, &demo_genesis_state(canonical));
    let mut state = demo_genesis_state(canonical);
    state.controllers[0].xonly = demo_keys().controller_1.xonly;
    let fork = genesis_with(demo_keys().root, &state);
    let legit_id = entity_id(&legit.payload);
    let fork_id = entity_id(&fork.payload);
    println!("legit_entity_id={}", hex::encode(legit_id));
    println!("fork_entity_id={}", hex::encode(fork_id));
    println!("ids_differ={}", legit_id != fork_id);
    println!("signer_entity_zero={}", legit.payload[5..37] == [0u8; 32]);
    Ok(())
}

pub fn write_fork(data_dir: &Path, outpoint: &str) -> Result<()> {
    let parsed = Outpoint::from_str(outpoint)?;
    let mut state = demo_genesis_state(canonical_outpoint(parsed));
    state.controllers[0].xonly = demo_keys().controller_1.xonly;
    let fork = genesis_with(demo_keys().root, &state);
    std::fs::create_dir_all(data_dir)?;
    write_object(&data_dir.join("genesis.o2a"), &fork)?;
    println!("fork_entity_id={}", hex::encode(entity_id(&fork.payload)));
    Ok(())
}

fn history_entity(data_dir: &Path) -> Result<[u8; 32]> {
    let genesis = read_signed(&data_dir.join("genesis.o2a"))?;
    Ok(entity_id(&genesis.payload))
}

fn read_signed(path: &Path) -> Result<o2a_demo_core::SignedObject> {
    let fields = crate::read_fields(path)?;
    let tag = match crate::field(&fields, "tag")? {
        "O2A/v0.1/entity-genesis" => "O2A/v0.1/entity-genesis",
        "O2A/v0.1/identity-transition" => "O2A/v0.1/identity-transition",
        "O2A/v0.1/recovery" => "O2A/v0.1/recovery",
        "O2A/v0.1/claim" => "O2A/v0.1/claim",
        other => bail!("unknown O2A tag {other}"),
    };
    let payload = hex::decode(crate::field(&fields, "payload")?)?;
    let digest = hex::decode(crate::field(&fields, "digest")?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("digest is not 32 bytes"))?;
    let signer_xonly = hex::decode(crate::field(&fields, "signer")?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signer is not 32 bytes"))?;
    let signature = hex::decode(crate::field(&fields, "signature")?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signature is not 64 bytes"))?;
    Ok(o2a_demo_core::SignedObject {
        tag,
        payload,
        digest,
        signer_xonly,
        signature,
    })
}

fn import_consignment(
    validator: &Path,
    stored: &std::collections::BTreeMap<String, String>,
    outpoint: &str,
) -> Result<&'static str> {
    let Some(consignment) = stored.get("consignment") else {
        return Ok("consignment absent");
    };
    let consignment = Path::new(consignment);
    if !consignment.exists() {
        fs::write(
            validator.join("import-error.txt"),
            format!("missing {consignment:?}"),
        )?;
        return Ok("consignment absent");
    }
    fs::create_dir_all(validator)?;
    let stockpile = rgb_persist_fs::StockpileDir::<bpstd::seals::TxoSeal>::load(
        validator.to_path_buf(),
        rgb::Consensus::Bitcoin,
        true,
    )?;
    let mut contracts: rgb::Contracts<rgb_persist_fs::StockpileDir<bpstd::seals::TxoSeal>> =
        rgb::Contracts::load(stockpile);
    let seal = external_seal(Outpoint::from_str(outpoint)?);
    if let Err(error) = contracts.consume_from_file(
        true,
        consignment,
        |_| {
            let mut seals = std::collections::BTreeMap::new();
            seals.insert(0, seal);
            seals
        },
        |_, _, _| Result::<_, std::convert::Infallible>::Ok(()),
    ) {
        fs::write(validator.join("import-error.txt"), format!("{error}"))?;
        return Ok("consignment import failed");
    }
    Ok("consignment imported")
}

fn fill_fee_tap(
    psbt: &mut bpstd::psbt::Psbt,
    descriptor: &RgbDescr<bpstd::XpubDerivable>,
    fee_script: &ScriptPubkey,
) -> Result<()> {
    for keychain in descriptor.keychains() {
        for index in 0u16..40 {
            let index = NormalIndex::from(index);
            let Some(script) = descriptor
                .derive(keychain, index)
                .find(|derived| derived.to_script_pubkey() == *fee_script)
            else {
                continue;
            };
            let terminal = Terminal::new(keychain, index);
            let input = psbt.input_mut(1).context("fee input")?;
            input.tap_internal_key = script.to_internal_pk();
            input.tap_merkle_root = script.to_tap_root();
            input.tap_bip32_derivation = descriptor.xonly_keyset(terminal);
            println!("fee_terminal={keychain:?}/{index}");
            return Ok(());
        }
    }
    bail!("fee script does not match the RGB demo wallet descriptor")
}

pub fn show_stale() -> Result<()> {
    let keys = demo_keys();
    let current = [o2a_demo_core::key_id(1, keys.controller_1.xonly)];
    let stale = [o2a_demo_core::SealBinding {
        authorizing_key_id: o2a_demo_core::key_id(1, keys.controller_0.xonly),
        seal_xonly: keys.seal_controller_0.xonly,
    }];
    let mut state = demo_rotation_state(1, [9; 32], [1; 36], [2; 36]);
    state.controller_bindings = stale.to_vec();
    let history = entity_id(&genesis_with(keys.root, &demo_genesis_state([0; 36])).payload);
    let object = controller_rotation_with(keys.controller_0, history, [9; 32], &state);
    println!("stale_transition_digest={}", hex::encode(object.digest));
    let recovery = demo_genesis_state([0; 36]).recovery_bindings;
    let error = seal_bindings_valid(
        &state.controller_bindings,
        &recovery,
        &current,
        &o2a_demo_core::demo_recovery_policy().key_ids,
        &[],
    );
    println!("stale_binding={}", error.unwrap_err());
    Ok(())
}

pub fn wallet_sees(data_dir: &Path, electrum: &str, outpoint: &str) -> Result<()> {
    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let target = Outpoint::from_str(outpoint)?;
    let seen = runtime.wallet.utxos().any(|utxo| utxo == target);
    println!("wallet_contains_seal={seen}");
    Ok(())
}

pub fn mismatched_genesis(outpoint: &str) -> Result<()> {
    let parsed = Outpoint::from_str(outpoint)?;
    let object = genesis_with(
        demo_keys().root,
        &demo_genesis_state(canonical_outpoint(parsed)),
    );
    println!("digest={}", hex::encode(object.digest));
    println!("mismatch_genesis=built");
    let _ = object;
    Ok(())
}
