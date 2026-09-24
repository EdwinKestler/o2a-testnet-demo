use std::collections::BTreeMap;
use std::convert::Infallible;
use std::env;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use amplify::confinement::SmallOrdSet;
use amplify::ByteArray;
use anyhow::{bail, Context, Result};
use bpstd::psbt::{Beneficiary, PsbtConstructor, TxParams};
use bpstd::seals::{TxoSeal, TxoSealExt, WOutpoint, WTxoSeal};
use bpstd::signers::TestnetSigner;
use bpstd::{h, Keychain, Network, Outpoint, Sats, ScriptPubkey, XprivAccount, XpubDerivable};
use o2a_demo_core::{controller_rotation as o2a_rotation, demo_keys, genesis as o2a_genesis};
use o2a_demo_rgb::{controller_rotation, demo_issuer, genesis_params, GenesisInput, RotationInput};
use rgb::popls::bp::{Prefab, PrefabBundle, WalletProvider};
use rgb::{CellAddr, Consensus, ContractId, Contracts, RgbSealDef};
use rgb_persist_fs::StockpileDir;
use rgbp::descriptors::RgbDescr;
use rgbp::resolvers::{ElectrumResolver, Resolver};
use rgbp::{FileHolder, Owner, RgbRuntime, RgbpRuntimeDir};
use rgpsbt::RgbPsbt;
use strict_encoding::StrictDumb;
use strict_types::StrictVal;

const ACK: &str = "--ack-disposable";

fn unsafe_seed() -> Vec<u8> {
    hex::decode(o2a_demo_core::UNSAFE_BIP39_SEED_HEX).expect("published unsafe seed")
}

fn payment_account() -> XprivAccount {
    XprivAccount::with_seed(true, &unsafe_seed()).derive(&h![86, 1, 0])
}

fn descriptor() -> RgbDescr<XpubDerivable> {
    let account = payment_account();
    let xpub = XpubDerivable::with(
        account.to_xpub_account(),
        &[Keychain::INNER, Keychain::OUTER],
    );
    let noise = xpub.xpub().chain_code().to_byte_array();
    RgbDescr::key_only_unfunded(xpub, noise)
}

fn runtime(data_dir: &Path, electrum: &str) -> Result<RgbpRuntimeDir<ElectrumResolver>> {
    fs::create_dir_all(data_dir)?;
    let holder = FileHolder::load(data_dir.join("wallet"))?;
    let resolver = ElectrumResolver::new(electrum)?;
    let owner = Owner::with_components(Network::Regtest, holder, resolver);
    let stockpile =
        StockpileDir::<TxoSeal>::load(data_dir.to_path_buf(), Consensus::Bitcoin, true)?;
    Ok(RgbRuntime::with_components(
        owner,
        Contracts::load(stockpile),
    ))
}

fn require_ack(args: &[String]) -> Result<()> {
    if !args.iter().any(|arg| arg == ACK) {
        bail!("refusing to create or change an identity without {ACK}")
    }
    eprintln!("DISPOSABLE IDENTITY: never use this demo identity or seed for funds or production");
    Ok(())
}

fn canonical_outpoint(outpoint: Outpoint) -> [u8; 36] {
    let mut bytes = [0u8; 36];
    bytes[..32].copy_from_slice(&outpoint.txid.to_byte_array());
    bytes[32..].copy_from_slice(&outpoint.vout_u32().to_le_bytes());
    bytes
}

fn outpoint_from_canonical(bytes: [u8; 36]) -> Outpoint {
    let txid_bytes: [u8; 32] = bytes[..32].try_into().expect("fixed transaction id length");
    let vout = u32::from_le_bytes(bytes[32..].try_into().expect("fixed vout length"));
    Outpoint::new(txid_bytes.into(), vout)
}

fn external_seal(outpoint: Outpoint) -> WTxoSeal {
    WTxoSeal {
        primary: WOutpoint::Extern(outpoint),
        secondary: TxoSealExt::strict_dumb(),
    }
}

fn write_object(path: &Path, object: &o2a_demo_core::SignedObject) -> Result<()> {
    let body = format!(
        "tag={}\npayload={}\ndigest={}\nsigner={}\nsignature={}\n",
        object.tag,
        hex::encode(&object.payload),
        hex::encode(object.digest),
        hex::encode(object.signer_xonly),
        hex::encode(object.signature),
    );
    fs::write(path, body)?;
    Ok(())
}

fn read_fields(path: &Path) -> Result<BTreeMap<String, String>> {
    let text = fs::read_to_string(path)?;
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.to_owned(), value.to_owned());
        }
    }
    Ok(fields)
}

fn field<'a>(map: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str> {
    map.get(name)
        .map(String::as_str)
        .with_context(|| format!("missing field {name}"))
}

fn prepare(data_dir: &Path) -> Result<()> {
    if data_dir.join("wallet").exists() {
        bail!("demo wallet already exists at {}", data_dir.display())
    }
    fs::create_dir_all(data_dir)?;
    let holder = FileHolder::create(data_dir.join("wallet"), descriptor())?;
    let resolver = ElectrumResolver::new("tcp://electrs:50001")?;
    let mut owner = Owner::with_components(Network::Regtest, holder, resolver);
    let genesis = owner.next_address();
    let successor = owner.next_address();
    println!("genesis_funding_address={genesis}");
    println!("successor_funding_address={successor}");
    Ok(())
}

fn issue(data_dir: &Path, electrum: &str) -> Result<()> {
    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let mut utxos = runtime.wallet.utxos().collect::<Vec<_>>();
    utxos.sort();
    if utxos.len() != 2 {
        bail!(
            "expected exactly two funded demo UTXOs, found {}",
            utxos.len()
        )
    }
    let genesis_seal = utxos[0];
    let successor_seal = utxos[1];
    let object = o2a_genesis(canonical_outpoint(genesis_seal));
    o2a_demo_core::verify(&object).map_err(anyhow::Error::msg)?;
    let keys = demo_keys();

    runtime.contracts.import_issuer(demo_issuer())?;
    let contract_id = runtime.issue(genesis_params(GenesisInput {
        root_xonly: keys.root.xonly,
        entity_id: o2a_demo_core::entity_id(keys.root.xonly),
        controller_xonly: keys.controller_0.xonly,
        policy_hash: o2a_demo_core::recovery_policy_hash(keys.recovery_0.xonly),
        state_commitment: object.digest,
        seal: genesis_seal,
    }))?;
    let state = runtime.contracts.contract_state(contract_id);
    let owned = state
        .owned
        .values()
        .flat_map(|items| items.iter())
        .next()
        .context("issued contract has no owned identity state")?;
    write_object(&data_dir.join("genesis.o2a"), &object)?;
    fs::write(
        data_dir.join("state.txt"),
        format!(
            "contract_id={contract_id}\ngenesis_cell={}\ngenesis_seal={genesis_seal}\nsuccessor_seal={successor_seal}\ngenesis_digest={}\n",
            owned.addr,
            hex::encode(object.digest),
        ),
    )?;
    println!("contract_id={contract_id}");
    println!("genesis_cell={}", owned.addr);
    println!("genesis_seal={genesis_seal}");
    println!("successor_seal={successor_seal}");
    println!("o2a_genesis=valid");
    println!("rgb_genesis=valid");
    Ok(())
}

fn rotate(data_dir: &Path, electrum: &str, consignment: &Path) -> Result<()> {
    let stored = read_fields(&data_dir.join("state.txt"))?;
    let contract_id = ContractId::from_str(field(&stored, "contract_id")?)?;
    let previous_cell = CellAddr::from_str(field(&stored, "genesis_cell")?)?;
    let genesis_seal = Outpoint::from_str(field(&stored, "genesis_seal")?)?;
    let successor_seal = Outpoint::from_str(field(&stored, "successor_seal")?)?;
    let prior_state = previous_cell.opid.to_byte_array();

    let object = o2a_rotation(
        prior_state,
        canonical_outpoint(genesis_seal),
        canonical_outpoint(successor_seal),
    );
    o2a_demo_core::verify_controller_rotation(&object).map_err(anyhow::Error::msg)?;
    let keys = demo_keys();
    let next = external_seal(successor_seal);

    let mut runtime = runtime(data_dir, electrum)?;
    runtime
        .update(1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let rotation = controller_rotation(RotationInput {
        previous_cell,
        controller_xonly: keys.controller_1.xonly,
        state_commitment: object.digest,
        next_seal: next,
    });
    let operation = runtime
        .contracts
        .contract_call(contract_id, rotation.params, rotation.seals)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let recipient = runtime.wallet.next_address();
    let (mut psbt, _meta) = runtime.wallet.construct_psbt(
        [genesis_seal],
        [Beneficiary::new(recipient, Sats::from_sats(10_000u64))],
        TxParams::with(Sats::from_sats(1_000u64)),
    )?;
    let host = psbt
        .insert_output(0, ScriptPubkey::op_return(&[]), Sats::ZERO)
        .map_err(|_| anyhow::anyhow!("unable to insert RGB commitment host"))?;
    host.set_opret_host()
        .map_err(|_| anyhow::anyhow!("unable to mark RGB commitment host"))?;
    psbt.complete_construction();

    let prefab = Prefab {
        closes: SmallOrdSet::try_from_iter([genesis_seal])?,
        defines: SmallOrdSet::new(),
        operation,
    };
    let bundle = PrefabBundle::new([prefab])?;
    psbt.rgb_fill_csv(&bundle)?;
    let mut psbt = runtime.complete(psbt, &bundle)?;
    let signer = TestnetSigner::new(payment_account());
    let signatures = psbt.sign(&signer)?;
    let finalized = psbt.finalize(runtime.wallet.descriptor());
    let tx = psbt
        .extract()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let anchor_txid = tx.txid();
    ElectrumResolver::new(electrum)?.broadcast(&tx)?;

    if consignment.exists() {
        bail!(
            "refusing to overwrite consignment {}",
            consignment.display()
        )
    }
    runtime
        .contracts
        .consign_to_file(consignment, contract_id, [next.auth_token()])?;
    let transition_path = consignment.with_extension("o2a");
    write_object(&transition_path, &object)?;
    fs::write(
        data_dir.join("rotation.txt"),
        format!(
            "anchor_txid={anchor_txid}\ntransition_digest={}\ntransition_o2a={}\nsuccessor_seal={successor_seal}\n",
            hex::encode(object.digest),
            transition_path.display(),
        ),
    )?;
    println!("anchor_txid={anchor_txid}");
    println!("bitcoin_input={genesis_seal}");
    println!("rgb_next_seal={successor_seal}");
    println!("o2a_transition=valid");
    println!("psbt_signatures={signatures}");
    println!("psbt_finalized_inputs={finalized}");
    println!("consignment={}", consignment.display());
    Ok(())
}

fn read_object(path: &Path) -> Result<o2a_demo_core::SignedObject> {
    let fields = read_fields(path)?;
    let payload = hex::decode(field(&fields, "payload")?)?;
    let digest = hex::decode(field(&fields, "digest")?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("digest is not 32 bytes"))?;
    let signer_xonly = hex::decode(field(&fields, "signer")?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signer is not 32 bytes"))?;
    let signature = hex::decode(field(&fields, "signature")?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signature is not 64 bytes"))?;
    if field(&fields, "tag")? != "O2A/v0.1/identity-transition" {
        bail!("unexpected O2A object tag")
    }
    Ok(o2a_demo_core::SignedObject {
        tag: "O2A/v0.1/identity-transition",
        payload,
        digest,
        signer_xonly,
        signature,
    })
}

fn verify_package(validator_dir: &Path, electrum: &str, package_dir: &Path) -> Result<()> {
    let consignment = package_dir.join("rotation.rgb");
    let object_path = package_dir.join("rotation.o2a");
    let object = read_object(&object_path)?;
    let successor = outpoint_from_canonical(
        o2a_demo_core::transition_next_seal(&object).map_err(anyhow::Error::msg)?,
    );
    fs::create_dir_all(validator_dir)?;
    let stockpile =
        StockpileDir::<TxoSeal>::load(validator_dir.to_path_buf(), Consensus::Bitcoin, true)?;
    let mut contracts: Contracts<StockpileDir<TxoSeal>> = Contracts::load(stockpile);
    let next = external_seal(successor);
    contracts
        .consume_from_file(
            true,
            &consignment,
            |_| {
                let mut seals = BTreeMap::new();
                seals.insert(0, next);
                seals
            },
            |_, _, _| Result::<_, Infallible>::Ok(()),
        )
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let resolver = ElectrumResolver::new(electrum)?;
    let height = resolver.last_block_height()?;
    contracts
        .update_witnesses(|txid| resolver.resolve_tx_status(txid), height, 1)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let contract_id = contracts
        .contract_ids()
        .next()
        .context("validator imported no contract")?;
    let state = contracts.contract_state(contract_id);
    let current = state
        .owned
        .values()
        .flat_map(|items| items.iter())
        .next()
        .context("validator found no current identity state")?;
    o2a_demo_core::verify_controller_rotation(&object).map_err(anyhow::Error::msg)?;
    match &current.assignment.data {
        StrictVal::Bytes(committed) if committed.as_slice() == object.digest => {}
        _ => bail!("RGB owned state does not commit to the O2A transition digest"),
    }
    println!("bitcoin_order_spend={:?}", current.status);
    println!("rgb_history=valid");
    println!("rgb_contract_id={contract_id}");
    println!("rgb_current_cell={}", current.addr);
    println!("rgb_current_seal={}", current.assignment.seal);
    println!("o2a_authorization=valid");
    println!("o2a_transition_digest={}", hex::encode(object.digest));
    Ok(())
}

fn help() {
    println!("o2a-demo create-identity prepare DATA_DIR {ACK}");
    println!("o2a-demo create-identity issue DATA_DIR ELECTRUM_URL {ACK}");
    println!("o2a-demo rotate-controller DATA_DIR ELECTRUM_URL CONSIGNMENT {ACK}");
    println!("o2a-demo verify-package VALIDATOR_DIR ELECTRUM_URL EVIDENCE_PACKAGE_DIR");
    println!("issue-attestation is intentionally out of scope for this pass");
}

fn run() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, action, data_dir, ack]
            if command == "create-identity" && action == "prepare" && ack == ACK =>
        {
            require_ack(&args)?;
            prepare(Path::new(data_dir))
        }
        [command, action, data_dir, electrum, ack]
            if command == "create-identity" && action == "issue" && ack == ACK =>
        {
            require_ack(&args)?;
            issue(Path::new(data_dir), electrum)
        }
        [command, data_dir, electrum, consignment, ack]
            if command == "rotate-controller" && ack == ACK =>
        {
            require_ack(&args)?;
            rotate(Path::new(data_dir), electrum, Path::new(consignment))
        }
        [command, validator, electrum, package] if command == "verify-package" => {
            verify_package(Path::new(validator), electrum, Path::new(package))
        }
        [command, ..] if command == "issue-attestation" => {
            bail!("attestations are out of scope for this pass")
        }
        _ => {
            help();
            Ok(())
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
