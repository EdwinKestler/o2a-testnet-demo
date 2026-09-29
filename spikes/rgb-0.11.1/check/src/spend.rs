//! Script-path PSBT construction for RGB 0.11.1. The fee key is local to this spike.

use o2a_demo_core::{DemoKey, SealScript, NUMS_X};
use psrgbt::RgbPsbtExt;
use rgbstd::bitcoin::absolute::LockTime;
use rgbstd::bitcoin::hashes::Hash;
use rgbstd::bitcoin::consensus::encode::serialize;
use rgbstd::bitcoin::key::{Keypair, Parity, TapTweak};
use rgbstd::bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use rgbstd::bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use rgbstd::bitcoin::taproot::{ControlBlock, LeafVersion, TapLeafHash, TapNodeHash, TaprootMerkleBranch};
use rgbstd::bitcoin::transaction::Version;
use rgbstd::bitcoin::{
    Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use rgbstd::containers::Fascia;
use rgbstd::txout::CloseMethod;
use rgbstd::{Operation, OpId, Transition, Txid};

pub struct LocalKey {
    pub keypair: Keypair,
    pub xonly: [u8; 32],
    pub script: ScriptBuf,
}

pub fn local_key() -> Result<LocalKey, String> {
    let mut secret = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut secret))
        .map_err(|err| format!("urandom: {err}"))?;
    let secp = Secp256k1::new();
    let parsed = SecretKey::from_slice(&secret).map_err(|err| err.to_string())?;
    let keypair = Keypair::from_secret_key(&secp, &parsed);
    let (xonly, _) = keypair.x_only_public_key();
    let script = ScriptBuf::new_p2tr(&secp, xonly, None);
    Ok(LocalKey {
        keypair,
        xonly: xonly.serialize(),
        script,
    })
}

pub enum LeafSpend {
    Controller { key: DemoKey },
    Recovery { keys: [DemoKey; 3] },
}

pub struct CommittedSpend {
    pub tx: Transaction,
    pub txid: Txid,
    pub hex: String,
    pub fascia: Fascia,
    pub transition_id: OpId,
}

pub fn commit_opret(
    seal_outpoint: OutPoint,
    seal_value: Amount,
    seal: &SealScript,
    leaf: LeafSpend,
    fee_outpoint: OutPoint,
    fee_value: Amount,
    fee_key: &Keypair,
    next_script: ScriptBuf,
    next_value: Amount,
    change_script: ScriptBuf,
    change_value: Amount,
    seal_sequence: u32,
    transition: Transition,
) -> Result<CommittedSpend, String> {
    let transition_id = transition.id();
    let seal_spk = ScriptBuf::from_bytes(seal.script_pubkey.clone());
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![
            TxIn {
                previous_output: seal_outpoint,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::from_consensus(seal_sequence),
                witness: Witness::default(),
            },
            TxIn {
                previous_output: fee_outpoint,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            },
        ],
        output: vec![
            TxOut {
                value: Amount::ZERO,
                script_pubkey: ScriptBuf::from_bytes(vec![0x6a]),
            },
            TxOut {
                value: next_value,
                script_pubkey: next_script,
            },
            TxOut {
                value: change_value,
                script_pubkey: change_script,
            },
        ],
    };
    let mut psbt = rgbstd::bitcoin::Psbt::from_unsigned_tx(tx).map_err(|err| err.to_string())?;
    psbt.inputs[0].witness_utxo = Some(TxOut {
        value: seal_value,
        script_pubkey: seal_spk.clone(),
    });
    psbt.inputs[1].witness_utxo = Some(TxOut {
        value: fee_value,
        script_pubkey: fee_script(fee_key),
    });
    let secp = Secp256k1::new();
    let internal = rgbstd::bitcoin::XOnlyPublicKey::from_slice(&NUMS_X).map_err(|err| err.to_string())?;
    psbt.inputs[0].tap_internal_key = Some(internal);
    psbt.inputs[0].tap_merkle_root = Some(
        TapNodeHash::from_slice(&seal.merkle_root).map_err(|err| err.to_string())?,
    );
    psbt.set_rgb_close_method(CloseMethod::OpretFirst);
    psbt.set_opret_host();
    psbt.set_as_unmodifiable();
    psbt.push_rgb_transition(transition).map_err(|err| err.to_string())?;
    let fascia = psbt.rgb_commit().map_err(|err| format!("rgb_commit: {err}"))?;
    let committed = psbt.unsigned_tx.clone();
    if committed.compute_txid() != fascia.witness_id() {
        return Err(format!(
            "fascia witness {} differs from committed tx {}",
            fascia.witness_id(),
            committed.compute_txid()
        ));
    }
    let prevouts = [
        TxOut {
            value: seal_value,
            script_pubkey: seal_spk,
        },
        TxOut {
            value: fee_value,
            script_pubkey: fee_script(fee_key),
        },
    ];
    let seal_witness = seal_witness(&committed, 0, seal, &leaf, &prevouts)?;
    let fee_witness = fee_witness(&committed, 1, fee_key, &prevouts)?;
    psbt.inputs[0].final_script_sig = Some(ScriptBuf::new());
    psbt.inputs[0].final_script_witness = Some(seal_witness);
    psbt.inputs[1].final_script_sig = Some(ScriptBuf::new());
    psbt.inputs[1].final_script_witness = Some(fee_witness);
    let tx = psbt.extract_tx().map_err(|err| format!("extract: {err:?}"))?;
    if tx.compute_txid() != fascia.witness_id() {
        return Err("signed txid moved after the commitment".into());
    }
    let hex = hex::encode(serialize(&tx));
    Ok(CommittedSpend {
        txid: tx.compute_txid(),
        tx,
        hex,
        fascia,
        transition_id,
    })
}

pub fn plain_spend(
    seal_outpoint: OutPoint,
    seal_value: Amount,
    seal: &SealScript,
    key: DemoKey,
    pay_script: ScriptBuf,
    pay_value: Amount,
) -> Result<(Transaction, String), String> {
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: seal_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: pay_value,
            script_pubkey: pay_script,
        }],
    };
    let prevouts = [TxOut {
        value: seal_value,
        script_pubkey: ScriptBuf::from_bytes(seal.script_pubkey.clone()),
    }];
    let witness = seal_witness(&tx, 0, seal, &LeafSpend::Controller { key }, &prevouts)?;
    let mut tx = tx;
    tx.input[0].witness = witness;
    let hex = hex::encode(serialize(&tx));
    Ok((tx, hex))
}

pub fn tapret_probe(transition: Transition, with_tree: bool) -> String {
    let secp = Secp256k1::new();
    let internal = match local_key() {
        Ok(key) => key,
        Err(err) => return format!("tapret setup failed: {err}"),
    };
    let (xonly, _) = internal.keypair.x_only_public_key();
    let script = if with_tree {
        let leaf = ScriptBuf::from_bytes(vec![0x51]);
        ScriptBuf::new_p2tr(&secp, xonly, Some(TapNodeHash::from_script(&leaf, LeafVersion::TapScript)))
    } else {
        ScriptBuf::new_p2tr(&secp, xonly, None)
    };
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: script,
        }],
    };
    let mut psbt = match rgbstd::bitcoin::Psbt::from_unsigned_tx(tx) {
        Ok(psbt) => psbt,
        Err(err) => return format!("tapret psbt: {err}"),
    };
    psbt.outputs[0].tap_internal_key = Some(xonly);
    if with_tree {
        let leaf = ScriptBuf::from_bytes(vec![0x51]);
        psbt.outputs[0].tap_tree = Some(
            rgbstd::bitcoin::taproot::TaprootBuilder::new()
                .add_leaf(0, leaf)
                .expect("leaf")
                .try_into_taptree()
                .expect("tree"),
        );
    }
    psrgbt::RgbOutExt::set_tapret_host(&mut psbt.outputs[0]);
    psbt.set_rgb_close_method(CloseMethod::TapretFirst);
    psbt.set_as_unmodifiable();
    if let Err(err) = psbt.push_rgb_transition(transition) {
        return format!("tapret embed: {err}");
    }
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| psbt.rgb_commit()));
    match outcome {
        Ok(Ok(fascia)) => format!(
            "ok witness={} proof={:?}",
            fascia.witness_id(),
            fascia.seal_witness().dbc_proof
        ),
        Ok(Err(err)) => format!("err {err}"),
        Err(_) => "panic".into(),
    }
}

fn fee_script(fee_key: &Keypair) -> ScriptBuf {
    let secp = Secp256k1::new();
    let (xonly, _) = fee_key.x_only_public_key();
    ScriptBuf::new_p2tr(&secp, xonly, None)
}

fn seal_witness(
    tx: &Transaction,
    input_index: usize,
    seal: &SealScript,
    leaf: &LeafSpend,
    prevouts: &[TxOut],
) -> Result<Witness, String> {
    let leaf_index = match leaf {
        LeafSpend::Controller { .. } => 0,
        LeafSpend::Recovery { .. } => 1,
    };
    let script = ScriptBuf::from_bytes(seal.scripts[leaf_index].clone());
    let sighash = SighashCache::new(tx)
        .taproot_script_spend_signature_hash(
            input_index,
            &Prevouts::All(prevouts),
            TapLeafHash::from_script(&script, LeafVersion::TapScript),
            TapSighashType::Default,
        )
        .map_err(|err| err.to_string())?;
    let message = sighash.to_byte_array();
    let mut stack: Vec<Vec<u8>> = Vec::new();
    match leaf {
        LeafSpend::Controller { key } => {
            if script.as_bytes().get(1..33) != Some(&key.xonly) {
                return Err("controller leaf key does not match the seal script".into());
            }
            stack.push(key.sign_schnorr(message).to_vec());
        }
        LeafSpend::Recovery { keys } => {
            let mut ordered = *keys;
            ordered.sort_by_key(|key| key.xonly);
            let mut signatures = Vec::new();
            for (index, key) in ordered.iter().enumerate() {
                if index < 2 {
                    signatures.push(key.sign_schnorr(message).to_vec());
                } else {
                    signatures.push(Vec::new());
                }
            }
            for signature in signatures.into_iter().rev() {
                stack.push(signature);
            }
        }
    }
    let control = control_block(seal, leaf_index)?;
    let secp = Secp256k1::new();
    let output = rgbstd::bitcoin::XOnlyPublicKey::from_slice(&seal.output_key).map_err(|err| err.to_string())?;
    if !control.verify_taproot_commitment(&secp, output, &script) {
        return Err("control block does not match the seal output key".into());
    }
    stack.push(script.as_bytes().to_vec());
    stack.push(control.serialize());
    let mut witness = Witness::new();
    for item in stack {
        witness.push(item);
    }
    Ok(witness)
}

fn control_block(seal: &SealScript, leaf_index: usize) -> Result<ControlBlock, String> {
    let nodes = seal.paths[leaf_index]
        .iter()
        .map(|hash| TapNodeHash::from_slice(hash).map_err(|err| err.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ControlBlock {
        leaf_version: LeafVersion::TapScript,
        output_key_parity: if seal.output_parity_odd {
            Parity::Odd
        } else {
            Parity::Even
        },
        internal_key: rgbstd::bitcoin::XOnlyPublicKey::from_slice(&NUMS_X).map_err(|err| err.to_string())?,
        merkle_branch: TaprootMerkleBranch::try_from(nodes).map_err(|err| err.to_string())?,
    })
}

fn fee_witness(
    tx: &Transaction,
    input_index: usize,
    fee_key: &Keypair,
    prevouts: &[TxOut],
) -> Result<Witness, String> {
    let sighash = SighashCache::new(tx)
        .taproot_key_spend_signature_hash(input_index, &Prevouts::All(prevouts), TapSighashType::Default)
        .map_err(|err| err.to_string())?;
    let secp = Secp256k1::new();
    let tweaked = fee_key.tap_tweak(&secp, None);
    let signature = secp.sign_schnorr_no_aux_rand(
        &Message::from_digest(sighash.to_byte_array()),
        tweaked.as_keypair(),
    );
    let mut witness = Witness::new();
    witness.push(signature.as_ref());
    Ok(witness)
}
