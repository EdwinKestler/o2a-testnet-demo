//! Disposable regtest smoke test for custom tapscript seals.
//!
//! SMOKE-ONLY, NON-NORMATIVE. Seal keys use m/9999'/1'/role'/index' from the
//! published unsafe demo seed. This is not an O2A identity role path and not
//! BIP86. No normative seal script is adopted.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use amplify::confinement::SmallOrdSet;
use amplify::hex::FromHex;
use amplify::ByteArray;
use anyhow::{bail, Context, Result};
use bpstd::psbt::{Psbt, PsbtConstructor, UnsignedTx, UnsignedTxIn};
use bpstd::seals::{TxoSeal, TxoSealExt, WOutpoint, WTxoSeal};
use bpstd::signers::TestnetSigner;
use bpstd::{
    ControlBlock, Derive, Descriptor, HardenedIndex, InternalPk, IntoTapHash, KeyOrigin,
    LeafScript, LeafVer, LockTime, NormalIndex, Outpoint, OutputPk, Parity, ScriptBytes,
    ScriptPubkey, SeqNo, SighashCache, TapBranchHash, TapDerivation, TapLeafHash, TapMerklePath,
    TapNodeHash, Terminal, Tx, TxOut, TxVer, VarIntArray, Witness, XOnlyPk, XprivAccount,
};
use rgb::RgbSealDef;
use o2a_demo_core::{controller_rotation as o2a_rotation, demo_keys, genesis as o2a_genesis};
use o2a_demo_rgb::{controller_rotation, demo_issuer, genesis_params, GenesisInput, RotationInput};
use rgb::popls::bp::{Prefab, PrefabBundle, WalletProvider};
use rgb::{CellAddr, Consensus, ContractId, Contracts};
use rgb_persist_fs::StockpileDir;
use rgbp::descriptors::RgbDescr;
use rgbp::resolvers::{ElectrumResolver, Resolver};
use rgbp::{FileHolder, Owner, RgbRuntime, RgbpRuntimeDir};
use rgpsbt::RgbPsbt;
use secp256k1::{Keypair, Message, SecretKey, Secp256k1};
use strict_encoding::StrictDumb;
use strict_types::StrictVal;

const NUMS_X: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
const SMOKE_PURPOSE: u32 = 9999;
const SMOKE_COIN: u32 = 1;
const ROLE_CONTROLLER: u32 = 1;
const ROLE_RECOVERY: u32 = 2;
const OP_CHECKSIG: u8 = 0xac;
const OP_CHECKSIGADD: u8 = 0xba;
const OP_NUMEQUALVERIFY: u8 = 0x9d;
const OP_CSV: u8 = 0xb2;

#[derive(Clone, Copy)]
struct SmokeKey {
    role: u32,
    index: u32,
    secret: [u8; 32],
    xonly: [u8; 32],
}

struct SealScript {
    name: &'static str,
    leaves: Vec<LeafScript>,
    root: TapNodeHash,
    output: OutputPk,
    parity: Parity,
    script_pubkey: ScriptPubkey,
    paths: Vec<Vec<TapNodeHash>>,
}

fn hardened(index: u32) -> u32 {
    index | (1 << 31)
}

fn master_xprv(seed: &[u8]) -> ([u8; 32], [u8; 32]) {
    use bitcoin_hashes::{hmac, sha512, Hash, HashEngine};
    let mut engine = hmac::HmacEngine::<sha512::Hash>::new(b"Bitcoin seed");
    engine.input(seed);
    let digest = hmac::Hmac::<sha512::Hash>::from_engine(engine).to_byte_array();
    (
        digest[..32].try_into().expect("hmac length"),
        digest[32..].try_into().expect("hmac length"),
    )
}

fn child_xprv(secret: [u8; 32], chain: [u8; 32], index: u32) -> ([u8; 32], [u8; 32]) {
    use bitcoin_hashes::{hmac, sha512, Hash, HashEngine};
    let mut data = [0u8; 37];
    if index >= 1 << 31 {
        data[1..33].copy_from_slice(&secret);
    } else {
        let key = SecretKey::from_slice(&secret).expect("parent");
        data[..33].copy_from_slice(
            &secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &key).serialize(),
        );
    }
    data[33..].copy_from_slice(&index.to_be_bytes());
    let mut engine = hmac::HmacEngine::<sha512::Hash>::new(&chain);
    engine.input(&data);
    let digest = hmac::Hmac::<sha512::Hash>::from_engine(engine).to_byte_array();
    let tweak = secp256k1::Scalar::from_be_bytes(digest[..32].try_into().expect("tweak"))
        .expect("smoke tweak in range");
    let parent = SecretKey::from_slice(&secret).expect("parent");
    let derived = parent.add_tweak(&tweak).expect("nonzero child");
    (
        derived.secret_bytes(),
        digest[32..].try_into().expect("chain"),
    )
}

fn smoke_key(role: u32, index: u32) -> SmokeKey {
    let seed = hex::decode(o2a_demo_core::UNSAFE_BIP39_SEED_HEX).expect("published seed");
    let (mut secret, mut chain) = master_xprv(&seed);
    for step in [
        hardened(SMOKE_PURPOSE),
        hardened(SMOKE_COIN),
        hardened(role),
        hardened(index),
    ] {
        (secret, chain) = child_xprv(secret, chain, step);
    }
    let _ = chain;
    let key = SecretKey::from_slice(&secret).expect("smoke secret");
    let pair = Keypair::from_secret_key(&Secp256k1::new(), &key);
    let (xonly, _) = pair.x_only_public_key();
    SmokeKey {
        role,
        index,
        secret,
        xonly: xonly.serialize(),
    }
}

fn smoke_account(key: &SmokeKey) -> XprivAccount {
    let seed = hex::decode(o2a_demo_core::UNSAFE_BIP39_SEED_HEX).expect("published seed");
    XprivAccount::with_seed(true, &seed).derive(&[
        HardenedIndex::from(SMOKE_PURPOSE as u16),
        HardenedIndex::from(SMOKE_COIN as u16),
        HardenedIndex::from(key.role as u16),
        HardenedIndex::from(key.index as u16),
    ])
}

fn push_data(buf: &mut Vec<u8>, data: &[u8]) {
    assert!(data.len() < 76, "smoke push is short");
    buf.push(data.len() as u8);
    buf.extend_from_slice(data);
}

fn checksig_leaf(key: &SmokeKey) -> LeafScript {
    let mut raw = Vec::new();
    push_data(&mut raw, &key.xonly);
    raw.push(OP_CHECKSIG);
    LeafScript::new(LeafVer::TapScript, ScriptBytes::from_checked(raw))
}

fn recovery_leaf(keys: &[SmokeKey]) -> LeafScript {
    let mut sorted = keys.to_vec();
    sorted.sort_by_key(|key| key.xonly);
    let mut raw = Vec::new();
    push_data(&mut raw, &sorted[0].xonly);
    raw.push(OP_CHECKSIG);
    for key in &sorted[1..] {
        push_data(&mut raw, &key.xonly);
        raw.push(OP_CHECKSIGADD);
    }
    // Core 31.1 compiles and_v(v:multi_a(2,R1,R2,R3),older(10)) to this tail.
    // The v: wrapper fuses to OP_NUMEQUALVERIFY, and this older() form has no OP_DROP.
    raw.push(0x52);
    raw.push(OP_NUMEQUALVERIFY);
    raw.push(0x5a);
    raw.push(OP_CSV);
    LeafScript::new(LeafVer::TapScript, ScriptBytes::from_checked(raw))
}

fn node_of_leaf(leaf: &LeafScript) -> TapNodeHash {
    TapLeafHash::with_leaf_script(leaf).into_tap_hash()
}

fn branch(left: TapNodeHash, right: TapNodeHash) -> TapNodeHash {
    TapBranchHash::with_nodes(left, right).into_tap_hash()
}

fn build_tree(leaves: Vec<LeafScript>) -> SealScript {
    let count = leaves.len();
    let mut nodes: Vec<TapNodeHash> = leaves.iter().map(node_of_leaf).collect();
    let mut members: Vec<Vec<usize>> = (0..count).map(|index| vec![index]).collect();
    let mut paths = vec![Vec::new(); count];
    while nodes.len() > 1 {
        let mut next_nodes = Vec::new();
        let mut next_members = Vec::new();
        let mut index = 0;
        while index < nodes.len() {
            if index + 1 == nodes.len() {
                next_nodes.push(nodes[index]);
                next_members.push(members[index].clone());
                index += 1;
                continue;
            }
            for &leaf in &members[index] {
                paths[leaf].push(nodes[index + 1]);
            }
            for &leaf in &members[index + 1] {
                paths[leaf].push(nodes[index]);
            }
            let mut combined = members[index].clone();
            combined.extend_from_slice(&members[index + 1]);
            next_nodes.push(branch(nodes[index], nodes[index + 1]));
            next_members.push(combined);
            index += 2;
        }
        nodes = next_nodes;
        members = next_members;
    }
    let internal = InternalPk::from_byte_array(
        hex::decode(NUMS_X)
            .expect("nums")
            .try_into()
            .expect("32"),
    )
    .expect("nums point");
    let root = nodes[0];
    let (output, parity) = internal.to_output_pk(Some(root));
    let script_pubkey = ScriptPubkey::p2tr_scripted(internal, root);
    SealScript {
        name: "",
        leaves,
        root,
        output,
        parity,
        script_pubkey,
        paths,
    }
}

fn seal_a() -> (Vec<SmokeKey>, SealScript) {
    let c1 = smoke_key(ROLE_CONTROLLER, 0);
    let recovery = recovery_keys();
    let mut script = build_tree(vec![checksig_leaf(&c1), recovery_leaf(&recovery)]);
    script.name = "A";
    (vec![c1], script)
}

fn seal_b() -> (Vec<SmokeKey>, SealScript) {
    let mut controllers = vec![smoke_key(ROLE_CONTROLLER, 0), smoke_key(ROLE_CONTROLLER, 1)];
    controllers.sort_by_key(|key| key.xonly);
    let recovery = recovery_keys();
    let leaves = vec![
        checksig_leaf(&controllers[0]),
        checksig_leaf(&controllers[1]),
        recovery_leaf(&recovery),
    ];
    let mut script = build_tree(leaves);
    script.name = "B";
    (controllers, script)
}

fn seal_c() -> (Vec<SmokeKey>, SealScript) {
    let c2 = smoke_key(ROLE_CONTROLLER, 1);
    let recovery = recovery_keys();
    let mut script = build_tree(vec![checksig_leaf(&c2), recovery_leaf(&recovery)]);
    script.name = "C";
    (vec![c2], script)
}

fn recovery_keys() -> Vec<SmokeKey> {
    let mut keys = vec![
        smoke_key(ROLE_RECOVERY, 0),
        smoke_key(ROLE_RECOVERY, 1),
        smoke_key(ROLE_RECOVERY, 2),
    ];
    keys.sort_by_key(|key| key.xonly);
    keys
}

fn hex_key(key: &SmokeKey) -> String {
    hex::encode(key.xonly)
}

fn descriptor_for(name: &str) -> String {
    let recovery = recovery_keys();
    let recovery_list = recovery
        .iter()
        .map(hex_key)
        .collect::<Vec<_>>()
        .join(",");
    let older = format!("and_v(v:multi_a(2,{recovery_list}),older(10))");
    let tree = match name {
        "A" => format!("{{pk({}),{older}}}", hex_key(&smoke_key(ROLE_CONTROLLER, 0))),
        "C" => format!("{{pk({}),{older}}}", hex_key(&smoke_key(ROLE_CONTROLLER, 1))),
        "B" => {
            let mut controllers = vec![smoke_key(ROLE_CONTROLLER, 0), smoke_key(ROLE_CONTROLLER, 1)];
            controllers.sort_by_key(|key| key.xonly);
            format!(
                "{{{{pk({}),pk({})}},{older}}}",
                hex_key(&controllers[0]),
                hex_key(&controllers[1])
            )
        }
        _ => unreachable!(),
    };
    format!("tr({NUMS_X},{tree})")
}

fn print_scripts() {
    println!("label=SMOKE-ONLY NON-NORMATIVE");
    println!("path_template=m/9999'/1'/role'/index'");
    println!("internal_key={NUMS_X}");
    for (role, index, name) in [
        (ROLE_CONTROLLER, 0, "C1"),
        (ROLE_CONTROLLER, 1, "C2"),
        (ROLE_RECOVERY, 0, "R1"),
        (ROLE_RECOVERY, 1, "R2"),
        (ROLE_RECOVERY, 2, "R3"),
    ] {
        let key = smoke_key(role, index);
        println!(
            "key {name} path=m/9999'/1'/{role}'/{index}' xonly={}",
            hex_key(&key)
        );
    }
    println!(
        "recovery_leaf={}",
        hex::encode(recovery_leaf(&recovery_keys()).as_script_bytes().as_slice())
    );
    for (name, script) in [("A", seal_a().1), ("B", seal_b().1), ("C", seal_c().1)] {
        println!("seal {name}");
        println!("descriptor={}", descriptor_for(name));
        println!("script_pubkey={:x}", script.script_pubkey);
        println!("merkle_root={}", script.root);
        println!("output_key={}", script.output.to_xonly_pk());
        println!("output_parity={:?}", script.parity);
        for (index, leaf) in script.leaves.iter().enumerate() {
            println!(
                "leaf_{index}={}",
                hex::encode(leaf.as_script_bytes().as_slice())
            );
        }
    }
}

fn payment_account() -> XprivAccount {
    let seed = hex::decode(o2a_demo_core::UNSAFE_BIP39_SEED_HEX).expect("seed");
    XprivAccount::with_seed(true, &seed).derive(&[
        HardenedIndex::from(86u16),
        HardenedIndex::from(1u16),
        HardenedIndex::from(0u16),
    ])
}

fn descriptor() -> RgbDescr<bpstd::XpubDerivable> {
    use amplify::ByteArray as _;
    use bpstd::{Keychain, XpubDerivable};
    let account = payment_account();
    let xpub = XpubDerivable::with(
        account.to_xpub_account(),
        &[Keychain::INNER, Keychain::OUTER],
    );
    let noise = xpub.xpub().chain_code().to_byte_array();
    RgbDescr::key_only_unfunded(xpub, noise)
}

fn external_seal(outpoint: Outpoint) -> WTxoSeal {
    WTxoSeal {
        primary: WOutpoint::Extern(outpoint),
        secondary: TxoSealExt::strict_dumb(),
    }
}

fn canonical_outpoint(outpoint: Outpoint) -> [u8; 36] {
    let mut bytes = [0u8; 36];
    bytes[..32].copy_from_slice(&outpoint.txid.to_byte_array());
    bytes[32..].copy_from_slice(&outpoint.vout_u32().to_le_bytes());
    bytes
}

fn runtime(data_dir: &Path, electrum: &str) -> Result<RgbpRuntimeDir<ElectrumResolver>> {
    let holder = FileHolder::load(data_dir.join("wallet"))?;
    let resolver = ElectrumResolver::new(electrum)?;
    let owner = Owner::with_components(bpstd::Network::Regtest, holder, resolver);
    let stockpile = StockpileDir::<TxoSeal>::load(data_dir.to_path_buf(), Consensus::Bitcoin, true)?;
    Ok(RgbRuntime::with_components(owner, Contracts::load(stockpile)))
}

fn prepare(data_dir: &Path) -> Result<()> {
    if data_dir.join("wallet").exists() {
        bail!("wallet exists");
    }
    fs::create_dir_all(data_dir)?;
    let holder = FileHolder::create(data_dir.join("wallet"), descriptor())?;
    let resolver = ElectrumResolver::new("tcp://electrs:50001")?;
    let mut owner = Owner::with_components(bpstd::Network::Regtest, holder, resolver);
    let first = owner.next_address();
    println!("fee_address_0={first}");
    println!("fee_script_0={:x}", first.script_pubkey());
    let second = owner.next_address();
    println!("fee_address_1={second}");
    println!("fee_script_1={:x}", second.script_pubkey());
    println!("DISPOSABLE IDENTITY: smoke wallet only, never use on mainnet");
    Ok(())
}

fn issue(data_dir: &Path, electrum: &str, seal: Outpoint) -> Result<()> {
    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let object = o2a_genesis(canonical_outpoint(seal));
    o2a_demo_core::verify(&object).map_err(anyhow::Error::msg)?;
    let keys = demo_keys();
    runtime.contracts.import_issuer(demo_issuer())?;
    let contract_id = runtime.issue(genesis_params(GenesisInput {
        root_xonly: keys.root.xonly,
        entity_id: o2a_demo_core::entity_id(keys.root.xonly),
        controller_xonly: keys.controller_0.xonly,
        policy_hash: o2a_demo_core::recovery_policy_hash(keys.recovery_0.xonly),
        state_commitment: object.digest,
        seal,
    }))?;
    let state = runtime.contracts.contract_state(contract_id);
    let owned = state
        .owned
        .values()
        .flat_map(|items| items.iter())
        .next()
        .context("no owned state")?;
    println!("contract_id={contract_id}");
    println!("genesis_cell={}", owned.addr);
    println!("genesis_seal={}", owned.assignment.seal);
    println!("o2a_digest={}", hex::encode(object.digest));
    fs::write(
        data_dir.join("state.txt"),
        format!(
            "contract_id={contract_id}\ncell={}\nseal={seal}\ndigest={}\n",
            owned.addr,
            hex::encode(object.digest)
        ),
    )?;
    fs::write(
        data_dir.join("genesis.o2a"),
        format!(
            "tag={}\npayload={}\ndigest={}\nsigner={}\nsignature={}\n",
            object.tag,
            hex::encode(&object.payload),
            hex::encode(object.digest),
            hex::encode(object.signer_xonly),
            hex::encode(object.signature)
        ),
    )?;
    Ok(())
}

fn show_utxos(data_dir: &Path, electrum: &str) -> Result<()> {
    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let mut utxos = runtime.wallet.utxos().collect::<Vec<_>>();
    utxos.sort();
    println!("utxo_count={}", utxos.len());
    for utxo in utxos {
        println!("utxo={utxo}");
    }
    Ok(())
}

fn try_select(data_dir: &Path, electrum: &str, outpoint: Outpoint) -> Result<()> {
    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let present = runtime.wallet.utxos().any(|utxo| utxo == outpoint);
    println!("wallet_contains={present}");
    let change = runtime.wallet.next_address();
    match runtime.wallet.construct_psbt(
        [outpoint],
        [bpstd::psbt::Beneficiary::new(
            change,
            bpstd::Sats::from_sats(10_000u64),
        )],
        bpstd::psbt::TxParams::with(bpstd::Sats::from_sats(1_000u64)),
    ) {
        Ok(_) => println!("construct_psbt=accepted"),
        Err(err) => println!("construct_psbt_error={err}"),
    }
    Ok(())
}

fn control_for(script: &SealScript, leaf: usize) -> ControlBlock {
    let internal = InternalPk::from_byte_array(
        hex::decode(NUMS_X).unwrap().try_into().unwrap(),
    )
    .unwrap();
    let siblings = script.paths[leaf]
        .iter()
        .map(|node| TapBranchHash::from(node.to_byte_array()))
        .collect::<Vec<_>>();
    ControlBlock::with(
        LeafVer::TapScript,
        internal,
        script.parity,
        TapMerklePath::try_from(siblings).expect("path length"),
    )
}

fn control_bytes(block: &ControlBlock) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut head = block.leaf_version.to_consensus_u8();
    if block.output_key_parity == Parity::Odd {
        head |= 1;
    }
    raw.push(head);
    raw.extend_from_slice(&block.internal_pk.to_byte_array());
    for step in &block.merkle_branch {
        raw.extend_from_slice(&step.to_byte_array());
    }
    raw
}

fn build_anchor(
    seal: Outpoint,
    seal_value: u64,
    seal_script: &ScriptPubkey,
    fee: Outpoint,
    fee_value: u64,
    fee_script: &ScriptPubkey,
    change_script: ScriptPubkey,
    fee_sats: u64,
    sequence: u32,
) -> Result<Psbt> {
    let change = seal_value
        .checked_add(fee_value)
        .and_then(|sum| sum.checked_sub(fee_sats))
        .context("inputs do not cover the fee")?;
    let unsigned = UnsignedTx {
        version: TxVer::V2,
        inputs: VarIntArray::from_iter_checked([
            UnsignedTxIn {
                prev_output: seal,
                sequence: SeqNo::from_consensus_u32(sequence),
            },
            UnsignedTxIn {
                prev_output: fee,
                sequence: SeqNo::from_consensus_u32(0xffff_ffff),
            },
        ]),
        outputs: VarIntArray::from_iter_checked([
            TxOut::new(ScriptPubkey::op_return(&[]), bpstd::Sats::ZERO),
            TxOut::new(change_script, bpstd::Sats::from_sats(change)),
        ]),
        lock_time: LockTime::ZERO,
    };
    let mut psbt = Psbt::from_tx(unsigned);
    psbt.input_mut(0).context("seal input")?.witness_utxo =
        Some(TxOut::new(seal_script.clone(), bpstd::Sats::from_sats(seal_value)));
    psbt.input_mut(1).context("fee input")?.witness_utxo =
        Some(TxOut::new(fee_script.clone(), bpstd::Sats::from_sats(fee_value)));
    let host = psbt.output_mut(0).context("opret")?;
    host.set_opret_host()
        .map_err(|_| anyhow::anyhow!("opret host"))?;
    psbt.complete_construction();
    let _ = (seal_script, fee_script);
    Ok(psbt)
}

fn fill_fee_tap(
    psbt: &mut Psbt,
    descriptor: &RgbDescr<bpstd::XpubDerivable>,
    fee_script: &ScriptPubkey,
) -> Result<()> {
    for keychain in descriptor.keychains() {
        for index in 0u16..20 {
            let index = NormalIndex::from(index);
            let Some(script) = descriptor
                .derive(keychain, index)
                .find(|script| script.to_script_pubkey() == *fee_script)
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

fn sign_fee(psbt: &mut Psbt) -> String {
    let signer = TestnetSigner::new(payment_account());
    match psbt.sign(&signer) {
        Ok(count) => format!("fee_signatures={count}"),
        Err(err) => format!("fee_sign_error={err}"),
    }
}

fn leaf_index_for(script: &SealScript, key: &SmokeKey) -> usize {
    script
        .leaves
        .iter()
        .position(|leaf| leaf.as_script_bytes().as_slice().windows(32).any(|window| window == key.xonly))
        .expect("leaf")
}

fn try_variant_a(psbt: &mut Psbt, key: &SmokeKey, leaf_hash: TapLeafHash) -> Result<String> {
    let account = smoke_account(key);
    let origin = match KeyOrigin::from_str(&account.origin().to_string()) {
        Ok(origin) => origin,
        Err(err) => return Ok(format!("variant_a_origin_error={err}")),
    };
    if let Some(input) = psbt.input_mut(0) {
        input.tap_bip32_derivation.insert(
            XOnlyPk::from_byte_array(key.xonly)?,
            TapDerivation {
                leaf_hashes: vec![leaf_hash],
                origin,
            },
        );
    }
    let signer = TestnetSigner::new_script_spent(leaf_hash, [account]);
    match psbt.sign(&signer) {
        Ok(count) => Ok(format!("variant_a_script_signatures={count}")),
        Err(err) => Ok(format!("variant_a_sign_error={err}")),
    }
}

fn manual_script_witness(
    psbt: &Psbt,
    input_index: usize,
    leaf: &LeafScript,
    control: &ControlBlock,
    signers: &[SmokeKey],
    positions: &[Option<SmokeKey>],
) -> Result<Witness> {
    let unsigned = psbt.to_unsigned_tx();
    let prevouts = psbt
        .inputs()
        .map(|input| {
            input
                .witness_utxo
                .clone()
                .context("missing witness utxo")
        })
        .collect::<Result<Vec<_>>>()?;
    let tx = Tx::from(unsigned);
    let mut cache = SighashCache::new(tx, prevouts)?;
    let leaf_hash = TapLeafHash::with_leaf_script(leaf);
    let sighash = cache.tap_sighash_script(input_index, leaf_hash, None)?;
    let secp = Secp256k1::new();
    let digest: [u8; 32] = sighash.into();
    let message = Message::from_digest(digest);
    let mut stack = Vec::new();
    for slot in positions.iter().rev() {
        match slot {
            Some(key) => {
                let secret = SecretKey::from_slice(&key.secret)?;
                let pair = Keypair::from_secret_key(&secp, &secret);
                let sig = secp.sign_schnorr_no_aux_rand(&message, &pair);
                stack.push(sig.as_ref().to_vec());
            }
            None => stack.push(Vec::new()),
        }
    }
    let _ = signers;
    stack.push(leaf.as_script_bytes().to_vec());
    stack.push(control_bytes(control));
    Ok(Witness::from_consensus_stack(stack))
}

fn anchor(
    data_dir: &Path,
    electrum: &str,
    consignment: &Path,
    previous: Outpoint,
    next: Outpoint,
    fee: Outpoint,
    fee_value: u64,
    seal_value: u64,
    fee_script_hex: &str,
    sequence: u32,
    leaf_kind: &str,
) -> Result<()> {
    let stored = fs::read_to_string(data_dir.join("state.txt"))?;
    let mut fields = BTreeMap::new();
    for line in stored.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.to_owned(), value.to_owned());
        }
    }
    let contract_id = ContractId::from_str(fields.get("contract_id").context("contract")?)?;
    let previous_cell = CellAddr::from_str(fields.get("cell").context("cell")?)?;
    let prior_state = previous_cell.opid.to_byte_array();
    let object = o2a_rotation(
        prior_state,
        canonical_outpoint(previous),
        canonical_outpoint(next),
    );
    o2a_demo_core::verify_controller_rotation(&object).map_err(anyhow::Error::msg)?;
    let keys = demo_keys();
    let next_seal = external_seal(next);
    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let rotation = controller_rotation(RotationInput {
        previous_cell,
        controller_xonly: keys.controller_1.xonly,
        state_commitment: object.digest,
        next_seal,
    });
    let operation = runtime
        .contracts
        .contract_call(contract_id, rotation.params, rotation.seals)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let (controllers, script) = if leaf_kind == "recovery" {
        seal_b()
    } else if previous == Outpoint::from_str(fields.get("seal").context("seal")?)? {
        seal_a()
    } else {
        seal_b()
    };
    let defined_cell = CellAddr::new(operation.opid(), 0);
    println!("defined_cell={defined_cell}");
    println!("defined_outputs={}", operation.destructible_out.len());
    let seal_spk = script.script_pubkey.clone();
    let fee_script = ScriptPubkey::from_hex(fee_script_hex).map_err(|err| anyhow::anyhow!(err))?;
    let change_script = runtime.wallet.next_address().script_pubkey();
    println!("change_script={change_script:x}");
    println!("change_value={}", seal_value + fee_value - 1_000);
    let mut psbt = build_anchor(
        previous,
        seal_value,
        &seal_spk,
        fee,
        fee_value,
        &fee_script,
        change_script,
        1_000,
        sequence,
    )?;
    let (leaf_pos, signing_keys, slots) = if leaf_kind == "recovery" {
        let recovery = recovery_keys();
        let leaf_pos = script.leaves.len() - 1;
        let slots = recovery
            .iter()
            .map(|key| {
                if key.index == 0 || key.index == 2 {
                    Some(*key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        (leaf_pos, recovery, slots)
    } else {
        let key = controllers
            .iter()
            .find(|key| key.index == 0 && key.role == ROLE_CONTROLLER)
            .copied()
            .context("C1")?;
        let leaf_pos = leaf_index_for(&script, &key);
        (leaf_pos, vec![key], vec![Some(key)])
    };
    let leaf = script.leaves[leaf_pos].clone();
    let leaf_hash = TapLeafHash::with_leaf_script(&leaf);
    let control = control_for(&script, leaf_pos);
    {
        let input = psbt.input_mut(0).context("seal")?;
        input.tap_internal_key = Some(control.internal_pk);
        input.tap_merkle_root = Some(script.root);
        input.tap_leaf_script.insert(control.clone(), leaf.clone());
    }
    println!(
        "variant_a_before_complete {}",
        try_variant_a(&mut psbt, &signing_keys[0], leaf_hash)?
    );
    if let Some(input) = psbt.input_mut(0) {
        input.tap_script_sig.clear();
        input.tap_key_sig = None;
    }
    let prefab = Prefab {
        closes: SmallOrdSet::try_from_iter([previous])?,
        defines: SmallOrdSet::new(),
        operation,
    };
    let bundle = PrefabBundle::new([prefab])?;
    psbt.rgb_fill_csv(&bundle)?;
    let mut psbt = runtime.complete(psbt, &bundle)?;
    {
        let input = psbt.input_mut(0).context("seal")?;
        input.tap_internal_key = Some(control.internal_pk);
        input.tap_merkle_root = Some(script.root);
        input.tap_leaf_script.insert(control.clone(), leaf.clone());
        println!("seal_leaf_scripts_after_complete={}", input.tap_leaf_script.len());
    }
    for key in slots.iter().flatten() {
        println!(
            "variant_a_after_complete {}",
            try_variant_a(&mut psbt, key, leaf_hash)?
        );
    }
    fill_fee_tap(&mut psbt, runtime.wallet.descriptor(), &fee_script)?;
    println!("{}", sign_fee(&mut psbt));
    let finalized = psbt.finalize(runtime.wallet.descriptor());
    println!("descriptor_finalized_inputs={finalized}");
    let seal_final = psbt.input(0).context("seal")?.is_finalized();
    println!("variant_a_seal_finalized={seal_final}");
    if !seal_final {
        let witness = manual_script_witness(&psbt, 0, &leaf, &control, &signing_keys, &slots)?;
        let input = psbt.input_mut(0).context("seal")?;
        // extract() requires final_script_sig even when the spend is witness-only.
        input.final_script_sig = Some(bpstd::SigScript::empty());
        input.final_witness = Some(witness);
        println!("variant_b_after_complete=set");
    }
    if !psbt.input(1).context("fee")?.is_finalized() {
        println!("fee_descriptor_finalized=false");
        let sig = psbt.input(1).context("fee")?.tap_key_sig.clone();
        if let Some(sig) = sig {
            let witness = Witness::from_consensus_stack(vec![sig.to_vec()]);
            let input = psbt.input_mut(1).context("fee")?;
            input.final_script_sig = Some(bpstd::SigScript::empty());
            input.final_witness = Some(witness);
            println!("fee_witness_from_tap_key_sig=set");
        } else {
            println!("fee_tap_key_sig=absent");
        }
    } else {
        println!("fee_descriptor_finalized=true");
    }
    let tx = psbt
        .extract()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    println!("raw_tx={tx:x}");
    println!("anchor_txid={}", tx.txid());
    println!("change_outpoint={}:1", tx.txid());
    if consignment.exists() {
        bail!("consignment exists");
    }
    runtime
        .contracts
        .consign_to_file(consignment, contract_id, [next_seal.auth_token()])?;
    let transition = consignment.with_extension("o2a");
    fs::write(
        &transition,
        format!(
            "tag={}\npayload={}\ndigest={}\nsigner={}\nsignature={}\n",
            object.tag,
            hex::encode(object.payload),
            hex::encode(object.digest),
            hex::encode(object.signer_xonly),
            hex::encode(object.signature)
        ),
    )?;
    fs::write(
        data_dir.join("state.txt"),
        format!(
            "contract_id={contract_id}\ncell={defined_cell}\nseal={next}\ndigest={}\n",
            hex::encode(object.digest)
        ),
    )?;
    println!("consignment={}", consignment.display());
    println!("o2a={}", transition.display());
    println!("leaf_kind={leaf_kind}");
    let _ = controllers;
    Ok(())
}

fn base58_encode(data: &[u8]) -> String {
    const ALPHA: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let leading = data.iter().take_while(|byte| **byte == 0).count();
    let mut digits = Vec::<u8>::new();
    for &byte in data {
        let mut carry = byte as u32;
        for digit in &mut digits {
            carry += u32::from(*digit) * 256;
            *digit = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut out = "1".repeat(leading);
    for digit in digits.iter().rev() {
        out.push(ALPHA[*digit as usize] as char);
    }
    out
}

fn regtest_wif(secret: &[u8; 32]) -> String {
    use bitcoin_hashes::{sha256, Hash};
    let mut payload = Vec::with_capacity(34);
    payload.push(0xef);
    payload.extend_from_slice(secret);
    payload.push(0x01);
    let checksum = sha256::Hash::hash(sha256::Hash::hash(&payload).as_ref());
    payload.extend_from_slice(&checksum[..4]);
    base58_encode(&payload)
}

fn print_wifs() {
    println!("label=SMOKE-ONLY NON-NORMATIVE regtest WIF");
    for (name, key) in [
        ("C1", smoke_key(ROLE_CONTROLLER, 0)),
        ("C2", smoke_key(ROLE_CONTROLLER, 1)),
        ("R1", smoke_key(ROLE_RECOVERY, 0)),
        ("R2", smoke_key(ROLE_RECOVERY, 1)),
        ("R3", smoke_key(ROLE_RECOVERY, 2)),
    ] {
        println!("wif {name} {}", regtest_wif(&key.secret));
    }
}

fn plain_spend(
    seal_name: &str,
    outpoint: Outpoint,
    seal_value: u64,
    destination_hex: &str,
    fee_sats: u64,
) -> Result<()> {
    let (controllers, script) = match seal_name {
        "A" => seal_a(),
        "B" => seal_b(),
        "C" => seal_c(),
        _ => bail!("seal name must be A, B, or C"),
    };
    let key = controllers
        .iter()
        .copied()
        .min_by_key(|key| key.index)
        .context("controller leaf")?;
    let destination =
        ScriptPubkey::from_hex(destination_hex).map_err(|err| anyhow::anyhow!(err))?;
    let value = seal_value
        .checked_sub(fee_sats)
        .context("fee exceeds the seal value")?;
    let unsigned = UnsignedTx {
        version: TxVer::V2,
        inputs: VarIntArray::from_iter_checked([UnsignedTxIn {
            prev_output: outpoint,
            sequence: SeqNo::from_consensus_u32(0xffff_ffff),
        }]),
        outputs: VarIntArray::from_iter_checked([TxOut::new(
            destination,
            bpstd::Sats::from_sats(value),
        )]),
        lock_time: LockTime::ZERO,
    };
    let mut psbt = Psbt::from_tx(unsigned);
    psbt.input_mut(0).context("seal")?.witness_utxo = Some(TxOut::new(
        script.script_pubkey.clone(),
        bpstd::Sats::from_sats(seal_value),
    ));
    psbt.complete_construction();
    let leaf_pos = leaf_index_for(&script, &key);
    let leaf = script.leaves[leaf_pos].clone();
    let control = control_for(&script, leaf_pos);
    {
        let input = psbt.input_mut(0).context("seal")?;
        input.tap_internal_key = Some(control.internal_pk);
        input.tap_merkle_root = Some(script.root);
        input.tap_leaf_script.insert(control.clone(), leaf.clone());
    }
    println!("psbt_version={:?}", psbt.version);
    println!("psbt_base64={}", psbt.to_base64());
    let witness = manual_script_witness(&psbt, 0, &leaf, &control, &[key], &[Some(key)])?;
    let input = psbt.input_mut(0).context("seal")?;
    input.final_script_sig = Some(bpstd::SigScript::empty());
    input.final_witness = Some(witness);
    let tx = psbt
        .extract()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    println!("plain_seal={seal_name}");
    println!("plain_leaf_key={}", hex_key(&key));
    println!("plain_has_rgb_commitment=false");
    println!("raw_tx={tx:x}");
    println!("txid={}", tx.txid());
    Ok(())
}

fn verify(validator_dir: &Path, electrum: &str, package_dir: &Path, o2a: bool) -> Result<()> {
    let consignment = package_dir.join("transition.rgb");
    let object_path = package_dir.join("transition.o2a");
    fs::create_dir_all(validator_dir)?;
    let stockpile = StockpileDir::<TxoSeal>::load(validator_dir.to_path_buf(), Consensus::Bitcoin, true)?;
    let mut contracts: Contracts<StockpileDir<TxoSeal>> = Contracts::load(stockpile);
    let fields = fs::read_to_string(&object_path)?;
    let mut map = BTreeMap::new();
    for line in fields.lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.to_owned(), v.to_owned());
        }
    }
    let payload = hex::decode(map.get("payload").context("payload")?)?;
    let object = o2a_demo_core::SignedObject {
        tag: "O2A/v0.1/identity-transition",
        payload: payload.clone(),
        digest: hex::decode(map.get("digest").context("digest")?)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("digest"))?,
        signer_xonly: hex::decode(map.get("signer").context("signer")?)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("signer"))?,
        signature: hex::decode(map.get("signature").context("signature")?)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("sig"))?,
    };
    let successor = o2a_demo_core::transition_next_seal(&object).map_err(anyhow::Error::msg)?;
    let mut raw = [0u8; 36];
    raw.copy_from_slice(&successor);
    let txid_bytes: [u8; 32] = raw[..32].try_into().unwrap();
    let vout = u32::from_le_bytes(raw[32..].try_into().unwrap());
    let next = external_seal(Outpoint::new(txid_bytes.into(), vout));
    contracts
        .consume_from_file(
            true,
            &consignment,
            |_| {
                let mut seals = BTreeMap::new();
                seals.insert(0, next);
                seals
            },
            |_, _, _| Result::<_, std::convert::Infallible>::Ok(()),
        )
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let resolver = ElectrumResolver::new(electrum)?;
    let height = resolver.last_block_height()?;
    println!("electrum_height={height}");
    contracts
        .update_witnesses(|txid| resolver.resolve_tx_status(txid), height, 1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let contract_id = contracts.contract_ids().next().context("no contract")?;
    let state = contracts.contract_state(contract_id);
    let current = state
        .owned
        .values()
        .flat_map(|items| items.iter())
        .next()
        .context("no cell")?;
    println!("rgb_contract_id={contract_id}");
    println!("rgb_current_cell={}", current.addr);
    println!("rgb_current_seal={}", current.assignment.seal);
    println!("bitcoin_order_spend={:?}", current.status);
    if o2a {
        o2a_demo_core::verify_controller_rotation(&object).map_err(anyhow::Error::msg)?;
        match &current.assignment.data {
            StrictVal::Bytes(committed) if committed.as_slice() == object.digest => {
                println!("o2a_authorization=valid");
            }
            _ => bail!("commitment mismatch"),
        }
    } else {
        println!("o2a_semantics=not_applicable");
    }
    Ok(())
}

fn observe(data_dir: &Path, electrum: &str) -> Result<()> {
    let mut runtime = runtime(data_dir, electrum)?;
    match runtime.update(1) {
        Ok(()) => println!("runtime_update=ok"),
        Err(err) => println!("runtime_update_error={err}"),
    }
    let stored = fs::read_to_string(data_dir.join("state.txt"))?;
    let contract_line = stored.lines().find(|line| line.starts_with("contract_id="));
    if let Some(line) = contract_line {
        let id = ContractId::from_str(line.trim_start_matches("contract_id="))?;
        let state = runtime.contracts.contract_state(id);
        for (name, cells) in &state.owned {
            for cell in cells {
                println!(
                    "owned name={name} cell={} seal={} status={:?}",
                    cell.addr, cell.assignment.seal, cell.status
                );
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [cmd] if cmd == "scripts" => {
            print_scripts();
            Ok(())
        }
        [cmd] if cmd == "wifs" => {
            print_wifs();
            Ok(())
        }
        [cmd, dir] if cmd == "prepare" => prepare(Path::new(dir)),
        [cmd, dir, electrum, seal] if cmd == "issue" => {
            issue(Path::new(dir), electrum, Outpoint::from_str(seal)?)
        }
        [cmd, dir, electrum] if cmd == "utxos" => show_utxos(Path::new(dir), electrum),
        [cmd, dir, electrum, outpoint] if cmd == "select" => {
            try_select(Path::new(dir), electrum, Outpoint::from_str(outpoint)?)
        }
        [cmd, dir, electrum, consignment, prev, next, fee, fee_value, seal_value, fee_script, sequence, kind]
            if cmd == "anchor" =>
        {
            anchor(
                Path::new(dir),
                electrum,
                Path::new(consignment),
                Outpoint::from_str(prev)?,
                Outpoint::from_str(next)?,
                Outpoint::from_str(fee)?,
                fee_value.parse()?,
                seal_value.parse()?,
                fee_script,
                sequence.parse()?,
                kind,
            )
        }
        [cmd, dir, electrum, package] if cmd == "verify" => {
            verify(Path::new(dir), electrum, Path::new(package), true)
        }
        [cmd, dir, electrum, package] if cmd == "verify-mechanism" => {
            verify(Path::new(dir), electrum, Path::new(package), false)
        }
        [cmd, dir, electrum] if cmd == "observe" => observe(Path::new(dir), electrum),
        [cmd, seal, outpoint, seal_value, destination, fee_sats] if cmd == "plain" => plain_spend(
            seal,
            Outpoint::from_str(outpoint)?,
            seal_value.parse()?,
            destination,
            fee_sats.parse()?,
        ),
        _ => {
            println!(
                "commands: scripts | prepare DIR | issue DIR ELECTRUM OUTPOINT | utxos DIR ELECTRUM | select DIR ELECTRUM OUTPOINT | anchor DIR ELECTRUM CONSIGNMENT PREV NEXT FEE FEE_SATS SEAL_SATS FEE_SCRIPT SEQUENCE KIND | verify DIR ELECTRUM PACKAGE | verify-mechanism DIR ELECTRUM PACKAGE | observe DIR ELECTRUM | plain SEAL OUTPOINT SEAL_SATS DEST_SCRIPT FEE_SATS"
            );
            Ok(())
        }
    }
}
