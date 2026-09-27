//! Script-path PSBT helpers.
//!
//! Finalization is manual. `extract` needs an empty `final_script_sig` on a
//! witness spend. Those are the bp-std workarounds from
//! `../o2a-protocol/docs/upstream-needs.md`.

use amplify::ByteArray;
use anyhow::{Context, Result};
use bpstd::psbt::{Psbt, UnsignedTx, UnsignedTxIn};
use bpstd::{
    ControlBlock, InternalPk, LeafScript, LeafVer, LockTime, Outpoint, Parity, ScriptBytes,
    ScriptPubkey, SeqNo, SighashCache, TapBranchHash, TapLeafHash, TapMerklePath, Tx, TxOut, TxVer,
    VarIntArray, Witness,
};
use o2a_demo_core::{seal_for_state, DemoKey, ResultingState, NUMS_X};

pub struct SpendRequest {
    pub seal: Outpoint,
    pub seal_value: u64,
    pub fee: Outpoint,
    pub fee_value: u64,
    pub fee_script: ScriptPubkey,
    pub change_script: ScriptPubkey,
    pub fee_sats: u64,
    pub state: ResultingState,
}

pub enum SpendStyle {
    Controller {
        signer: DemoKey,
    },
    Recovery {
        slots: Vec<Option<DemoKey>>,
    },
    ClosePlain {
        signer: DemoKey,
        destination: ScriptPubkey,
    },
    /// Recovery-leaf spend with no fee input and no RGB host.
    RecoveryClose {
        slots: Vec<Option<DemoKey>>,
        destination: ScriptPubkey,
    },
}

pub struct OpenSpend {
    pub psbt: Psbt,
    pub leaf: LeafScript,
    pub control: ControlBlock,
    pub slots: Vec<Option<DemoKey>>,
}

pub fn open_spend(request: &SpendRequest, style: SpendStyle) -> Result<OpenSpend> {
    let built = seal_for_state(&request.state).map_err(anyhow::Error::msg)?;
    let seal_script = ScriptPubkey::from_checked(built.script_pubkey.clone());
    let (sequence, leaf_index, slots) = match &style {
        SpendStyle::Controller { signer } | SpendStyle::ClosePlain { signer, .. } => {
            let leaf_index = built
                .scripts
                .iter()
                .position(|script| script.windows(32).any(|window| window == signer.xonly))
                .context("controller leaf")?;
            (0xffff_ffff, leaf_index, vec![Some(*signer)])
        }
        SpendStyle::Recovery { slots } | SpendStyle::RecoveryClose { slots, .. } => (
            request.state.recovery.delay_blocks,
            built.scripts.len() - 1,
            slots.clone(),
        ),
    };
    let single_input = matches!(
        style,
        SpendStyle::ClosePlain { .. } | SpendStyle::RecoveryClose { .. }
    );
    let destination = match &style {
        SpendStyle::ClosePlain { destination, .. }
        | SpendStyle::RecoveryClose { destination, .. } => destination.clone(),
        _ => ScriptPubkey::op_return(&[]),
    };
    let change_value = if single_input {
        request
            .seal_value
            .checked_sub(request.fee_sats)
            .context("fee exceeds the seal")?
    } else {
        request
            .seal_value
            .checked_add(request.fee_value)
            .and_then(|sum| sum.checked_sub(request.fee_sats))
            .context("inputs do not cover the fee")?
    };
    let (inputs, outputs) = if single_input {
        (
            vec![UnsignedTxIn {
                prev_output: request.seal,
                sequence: SeqNo::from_consensus_u32(sequence),
            }],
            vec![TxOut::new(
                destination,
                bpstd::Sats::from_sats(change_value),
            )],
        )
    } else {
        (
            vec![
                UnsignedTxIn {
                    prev_output: request.seal,
                    sequence: SeqNo::from_consensus_u32(sequence),
                },
                UnsignedTxIn {
                    prev_output: request.fee,
                    sequence: SeqNo::from_consensus_u32(0xffff_ffff),
                },
            ],
            vec![
                TxOut::new(destination, bpstd::Sats::ZERO),
                TxOut::new(
                    request.change_script.clone(),
                    bpstd::Sats::from_sats(change_value),
                ),
            ],
        )
    };
    let unsigned = UnsignedTx {
        version: TxVer::V2,
        inputs: VarIntArray::from_iter_checked(inputs),
        outputs: VarIntArray::from_iter_checked(outputs),
        lock_time: LockTime::ZERO,
    };
    let mut psbt = Psbt::from_tx(unsigned);
    psbt.input_mut(0).context("seal")?.witness_utxo = Some(TxOut::new(
        seal_script.clone(),
        bpstd::Sats::from_sats(request.seal_value),
    ));
    if !single_input {
        psbt.input_mut(1).context("fee")?.witness_utxo = Some(TxOut::new(
            request.fee_script.clone(),
            bpstd::Sats::from_sats(request.fee_value),
        ));
    }
    psbt.complete_construction();
    let leaf = LeafScript::new(
        LeafVer::TapScript,
        ScriptBytes::from_checked(built.scripts[leaf_index].clone()),
    );
    let internal = InternalPk::from_byte_array(NUMS_X).context("NUMS key")?;
    let parity = if built.output_parity_odd {
        Parity::Odd
    } else {
        Parity::Even
    };
    let siblings = built.paths[leaf_index]
        .iter()
        .map(|hash| TapBranchHash::from_byte_array(*hash))
        .collect::<Vec<_>>();
    let control = ControlBlock::with(
        LeafVer::TapScript,
        internal,
        parity,
        TapMerklePath::try_from(siblings).map_err(|_| anyhow::anyhow!("merkle path"))?,
    );
    {
        let input = psbt.input_mut(0).context("seal")?;
        input.tap_internal_key = Some(internal);
        input.tap_merkle_root = Some(TapBranchHash::from_byte_array(built.merkle_root).into());
        input.tap_leaf_script.insert(control.clone(), leaf.clone());
    }
    Ok(OpenSpend {
        psbt,
        leaf,
        control,
        slots,
    })
}

pub fn finish_spend(mut open: OpenSpend) -> Result<String> {
    let witness = manual_witness(&open.psbt, &open.leaf, &open.control, &open.slots)?;
    let input = open.psbt.input_mut(0).context("seal")?;
    input.final_script_sig = Some(bpstd::SigScript::empty());
    input.final_witness = Some(witness);
    let tx = open
        .psbt
        .extract()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    Ok(format!("{tx:x}"))
}

pub fn script_path_tx(request: &SpendRequest, style: SpendStyle) -> Result<String> {
    finish_spend(open_spend(request, style)?)
}

pub fn close_plain_tx(
    request: SpendRequest,
    signer: DemoKey,
    destination: ScriptPubkey,
) -> Result<String> {
    script_path_tx(
        &request,
        SpendStyle::ClosePlain {
            signer,
            destination,
        },
    )
}

fn manual_witness(
    psbt: &Psbt,
    leaf: &LeafScript,
    control: &ControlBlock,
    slots: &[Option<DemoKey>],
) -> Result<Witness> {
    let unsigned = psbt.to_unsigned_tx();
    let prevouts = psbt
        .inputs()
        .map(|input| input.witness_utxo.clone().context("witness utxo"))
        .collect::<Result<Vec<_>>>()?;
    let tx = Tx::from(unsigned);
    let mut cache = SighashCache::new(tx, prevouts)?;
    let leaf_hash = TapLeafHash::with_leaf_script(leaf);
    let sighash = cache.tap_sighash_script(0, leaf_hash, None)?;
    let digest: [u8; 32] = sighash.into();
    let mut stack = Vec::new();
    for slot in slots.iter().rev() {
        match slot {
            Some(key) => stack.push(key.sign_schnorr(digest).to_vec()),
            None => stack.push(Vec::new()),
        }
    }
    stack.push(leaf.as_script_bytes().to_vec());
    let mut control_raw = Vec::new();
    let mut head = control.leaf_version.to_consensus_u8();
    if control.output_key_parity == Parity::Odd {
        head |= 1;
    }
    control_raw.push(head);
    control_raw.extend_from_slice(&control.internal_pk.to_byte_array());
    for step in &control.merkle_branch {
        control_raw.extend_from_slice(&step.to_byte_array());
    }
    stack.push(control_raw);
    Ok(Witness::from_consensus_stack(stack))
}
