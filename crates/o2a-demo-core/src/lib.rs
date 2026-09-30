//! Deterministic, network-free O2A encoding and verification boundary.
//!
//! This is the demo's single implementation of the normative byte and signing
//! rules at [`SPEC_COMMIT`]. It accepts explicit state and chain evidence; it
//! never reads the network, clock, or a database.

mod chain;
mod decode;
mod derive;
mod encode;
mod eval;
mod seal;

use secp256k1::{schnorr::Signature, Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};

pub use chain::{
    evaluate_lineage, format_lineage_report, inclusion_matches, merkle_root, CurrentSealView,
    InclusionProof, LineageEvidence, LineageReport, SealFact,
};
pub use decode::{
    decode_identity_state, decode_payload, evaluate_name_claim, official_name_of,
    ClaimAuthorization,
};
pub use encode::{
    bytes_field, common_header, content_reference, encode_recovery_policy, encode_resulting_state,
    encode_seal_policy, entity_id as entity_id_of_payload, key_id, list_items, option_fixed,
    state_id as state_id_of, text_field, ControllerEntry, RecoveryPolicy, ResultingState,
    SealBinding,
};
pub use eval::{
    adapter_key_distinct, adapter_scheme, capability_known, evidence_ids_valid, genesis_root_ok,
    header_role_allowed, identity_history_state, increasing_expiry, manifest_binding,
    observation_time_status, package_object_gap, recovery_policy_valid, recovery_witness_status,
    revocation_target, seal_bindings_valid, seal_output_matches, seal_policy_valid,
    state_authorizes, HistoryInput, RecoveryClock, SealWatch,
};
pub use seal::{recovery_leaf, script_num, seal_script, SealScript, NUMS_X};

pub const SPEC_COMMIT: &str = "42fbb86532b9c16ae9c9d78c954d20e2cac9249d";
pub const CANONICAL_RULES: [&str; 4] = [
    "../o2a-protocol/specs/canonical-encoding.md",
    "../o2a-protocol/specs/cryptographic-profile.md",
    "../o2a-protocol/specs/key-derivation-profile.md",
    "../o2a-protocol/specs/rgb-identity-contract.md",
];
pub const UNSAFE_BIP39_SEED_HEX: &str = concat!(
    "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e5349553",
    "1f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04"
);
pub const NETWORK_REGTEST: u8 = 4;
pub const DEMO_RECOVERY_THRESHOLD: u16 = 2;
pub const DEMO_DELAY_BLOCKS: u32 = 10;

const GENESIS_TAG: &str = "O2A/v0.1/entity-genesis";
const TRANSITION_TAG: &str = "O2A/v0.1/identity-transition";
const RECOVERY_TAG: &str = "O2A/v0.1/recovery";
const CLAIM_TAG: &str = "O2A/v0.1/claim";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoKey {
    secret: [u8; 32],
    pub xonly: [u8; 32],
}

impl DemoKey {
    pub fn from_secret(secret: [u8; 32]) -> Result<Self, &'static str> {
        let parsed = SecretKey::from_slice(&secret).map_err(|_| "invalid secret")?;
        let pair = Keypair::from_secret_key(&Secp256k1::new(), &parsed);
        let (xonly, _) = pair.x_only_public_key();
        Ok(Self {
            secret,
            xonly: xonly.serialize(),
        })
    }

    pub fn sign_schnorr(&self, message: [u8; 32]) -> [u8; 64] {
        let secret = SecretKey::from_slice(&self.secret).expect("published demo key");
        let secp = Secp256k1::new();
        let pair = Keypair::from_secret_key(&secp, &secret);
        let signature = secp.sign_schnorr_no_aux_rand(&Message::from_digest(message), &pair);
        *signature.as_ref()
    }
}

/// Disposable regtest entity 0.
///
/// Seal indexes on `m/1'/0'/4'/index'` are paired in this order: controller 0,
/// controller 1, recovery 0, recovery 1, recovery 2. That allocation is a demo
/// default, not a second derivation rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoKeys {
    pub root: DemoKey,
    pub controller_0: DemoKey,
    pub controller_1: DemoKey,
    pub recovery_0: DemoKey,
    pub recovery_1: DemoKey,
    pub recovery_2: DemoKey,
    pub seal_controller_0: DemoKey,
    pub seal_controller_1: DemoKey,
    pub seal_recovery_0: DemoKey,
    pub seal_recovery_1: DemoKey,
    pub seal_recovery_2: DemoKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedObject {
    pub tag: &'static str,
    pub payload: Vec<u8>,
    pub digest: [u8; 32],
    pub signer_xonly: [u8; 32],
    pub signature: [u8; 64],
}

pub fn evaluation_boundary() -> &'static str {
    "explicit evidence in; deterministic three-layer result out; no network fetch"
}

pub fn unsafe_seed() -> Vec<u8> {
    hex::decode(UNSAFE_BIP39_SEED_HEX).expect("published seed hex")
}

/// Derives the disposable regtest entity 0 keys from the published unsafe seed.
pub fn demo_entity_index() -> u32 {
    std::env::var("O2A_DEMO_ENTITY")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

pub fn demo_keys() -> DemoKeys {
    keys_for(demo_coin())
}

/// Route B keys for an explicit coin type. `0` is mainnet. `1` is every other coin.
pub fn keys_for(coin: u32) -> DemoKeys {
    let seed = demo_seed();
    let entity = demo_entity_index();
    let key = |role: u32, index: u32| derive::identity_key(&seed, coin, entity, role, index);
    DemoKeys {
        root: key(0, 0),
        controller_0: key(1, 0),
        controller_1: key(1, 1),
        recovery_0: key(2, 0),
        recovery_1: key(2, 1),
        recovery_2: key(2, 2),
        seal_controller_0: key(4, 0),
        seal_controller_1: key(4, 1),
        seal_recovery_0: key(4, 2),
        seal_recovery_1: key(4, 3),
        seal_recovery_2: key(4, 4),
    }
}

pub fn identity_key(coin: u32, entity: u32, role: u32, index: u32) -> DemoKey {
    derive::identity_key(&unsafe_seed(), coin, entity, role, index)
}

pub fn accept_identity_key(
    profile_version: u16,
    network: &str,
    path: &str,
    candidate: &[u8; 32],
) -> Result<(), &'static str> {
    derive::accept_identity_key(profile_version, network, path, candidate, &unsafe_seed())
}

pub fn tagged_hash(tag: &str, payload: &[u8]) -> [u8; 32] {
    use bitcoin_hashes::{sha256, Hash, HashEngine};
    let tag_hash = sha256::Hash::hash(tag.as_bytes()).to_byte_array();
    let mut engine = sha256::Hash::engine();
    engine.input(&tag_hash);
    engine.input(&tag_hash);
    engine.input(payload);
    sha256::Hash::from_engine(engine).to_byte_array()
}

/// EntityID of one exact genesis payload. The payload's `signer_entity` is 32 zero bytes.
pub fn entity_id(genesis_payload: &[u8]) -> [u8; 32] {
    entity_id_of_payload(genesis_payload)
}

/// O2A state id: `TaggedHash("O2A/v0.1/state-id", entity_id || resulting_state)`.
pub fn state_id(entity: &[u8; 32], resulting_state: &[u8]) -> [u8; 32] {
    state_id_of(entity, resulting_state)
}

/// Network byte for demo objects. Mainnet is refused.
pub fn demo_network() -> u8 {
    match std::env::var("O2A_DEMO_NETWORK").ok().as_deref() {
        None | Some("") | Some("regtest") => NETWORK_REGTEST,
        Some("signet") => 3,
        Some("testnet") => 1,
        Some("testnet4") => 2,
        Some("mainnet") => panic!("mainnet identities are not authorized"),
        Some(other) => panic!("unknown O2A_DEMO_NETWORK {other}"),
    }
}

pub fn demo_coin() -> u32 {
    1
}

pub fn demo_delay_blocks() -> u32 {
    std::env::var("O2A_DEMO_DELAY")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEMO_DELAY_BLOCKS)
}

pub fn demo_threshold() -> u16 {
    std::env::var("O2A_DEMO_THRESHOLD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEMO_RECOVERY_THRESHOLD)
}

fn demo_seed() -> Vec<u8> {
    let Ok(path) = std::env::var("O2A_DEMO_SEED_FILE") else {
        return unsafe_seed();
    };
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("seed file {path}: {err}"));
    let compact: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let bytes = hex::decode(compact).unwrap_or_else(|err| panic!("seed file {path}: {err}"));
    if bytes.len() != 64 {
        panic!("seed file {path} must contain 64 bytes");
    }
    if bytes.as_slice() == unsafe_seed().as_slice() {
        panic!("seed file {path} is the published unsafe seed");
    }
    bytes
}

/// Rejects an unknown network or a root that is not a BIP340 x-only key.
///
/// The EntityID itself is [`entity_id`] of the genesis payload, not of the root.
pub fn entity_id_checked(network: u8, root: &[u8; 32]) -> Result<(), &'static str> {
    if !matches!(network, 0..=4) {
        return Err("unknown Bitcoin network");
    }
    if XOnlyPublicKey::from_slice(root).is_err() {
        return Err("root is not a BIP340 x-only public key");
    }
    Ok(())
}

pub fn recovery_policy_hash(policy: &RecoveryPolicy) -> [u8; 32] {
    tagged_hash("O2A/v0.1/recovery-policy", &encode_recovery_policy(policy))
}

pub fn recovery_policy_from(keys: &DemoKeys) -> RecoveryPolicy {
    let mut key_ids = [
        key_id(2, keys.recovery_0.xonly),
        key_id(2, keys.recovery_1.xonly),
        key_id(2, keys.recovery_2.xonly),
    ];
    key_ids.sort();
    RecoveryPolicy {
        version: 1,
        sequence: 1,
        threshold: demo_threshold(),
        key_ids: key_ids.to_vec(),
        delay_blocks: demo_delay_blocks(),
        cancellation_rule: 1,
    }
}

pub fn demo_recovery_policy() -> RecoveryPolicy {
    recovery_policy_from(&demo_keys())
}

pub fn demo_recovery_policy_bytes() -> Vec<u8> {
    encode_recovery_policy(&demo_recovery_policy())
}

fn binding(role: u8, authorizer: [u8; 32], seal: [u8; 32]) -> SealBinding {
    SealBinding {
        authorizing_key_id: key_id(role, authorizer),
        seal_xonly: seal,
    }
}

fn sort_bindings(bindings: &mut [SealBinding]) {
    bindings.sort_by_key(|binding| binding.bytes());
}

/// Genesis names controller 0 and the three recovery keys.
pub fn demo_genesis_state(next_seal: [u8; 36]) -> ResultingState {
    genesis_state_from(&demo_keys(), next_seal)
}

/// Genesis state for an explicit key set. The caller chooses the coin type.
pub fn genesis_state_from(keys: &DemoKeys, next_seal: [u8; 36]) -> ResultingState {
    let mut controller_bindings = [binding(
        1,
        keys.controller_0.xonly,
        keys.seal_controller_0.xonly,
    )];
    sort_bindings(&mut controller_bindings);
    let mut recovery_bindings = [
        binding(2, keys.recovery_0.xonly, keys.seal_recovery_0.xonly),
        binding(2, keys.recovery_1.xonly, keys.seal_recovery_1.xonly),
        binding(2, keys.recovery_2.xonly, keys.seal_recovery_2.xonly),
    ];
    sort_bindings(&mut recovery_bindings);
    ResultingState {
        sequence: 0,
        previous_state: None,
        previous_seal: None,
        next_seal,
        controllers: vec![ControllerEntry {
            xonly: keys.controller_0.xonly,
            capabilities: encode::demo_controller_capabilities(),
        }],
        recovery: recovery_policy_from(keys),
        controller_bindings: controller_bindings.to_vec(),
        recovery_bindings: recovery_bindings.to_vec(),
        lifecycle_status: 1,
    }
}

/// Controller rotation replaces the controller set with controller 1.
pub fn demo_rotation_state(
    sequence: u64,
    previous_state: [u8; 32],
    previous_seal: [u8; 36],
    next_seal: [u8; 36],
) -> ResultingState {
    let keys = demo_keys();
    let mut state = demo_genesis_state(next_seal);
    state.sequence = sequence;
    state.previous_state = Some(previous_state);
    state.previous_seal = Some(previous_seal);
    state.controllers = vec![ControllerEntry {
        xonly: keys.controller_1.xonly,
        capabilities: encode::demo_controller_capabilities(),
    }];
    state.controller_bindings = vec![binding(
        1,
        keys.controller_1.xonly,
        keys.seal_controller_1.xonly,
    )];
    sort_bindings(&mut state.controller_bindings);
    state
}

fn sign(tag: &'static str, payload: Vec<u8>, key: DemoKey) -> SignedObject {
    let digest = tagged_hash(tag, &payload);
    let secret = SecretKey::from_slice(&key.secret).expect("valid demo signing key");
    let pair = Keypair::from_secret_key(&Secp256k1::new(), &secret);
    let msg = Message::from_digest(digest);
    let signature = Secp256k1::new().sign_schnorr_no_aux_rand(&msg, &pair);
    SignedObject {
        tag,
        payload,
        digest,
        signer_xonly: key.xonly,
        signature: *signature.as_ref(),
    }
}

pub fn genesis_with(root: DemoKey, state: &ResultingState) -> SignedObject {
    genesis_for(demo_network(), root, state)
}

/// Genesis for an explicit network byte. The byte is the caller's profile value.
pub fn genesis_for(network: u8, root: DemoKey, state: &ResultingState) -> SignedObject {
    let mut payload = common_header(network, 1, [0u8; 32], None, key_id(0, root.xonly), 0, 1);
    payload.extend_from_slice(&2u16.to_le_bytes());
    payload.extend_from_slice(&root.xonly);
    payload.extend_from_slice(&encode_resulting_state(state));
    sign(GENESIS_TAG, payload, root)
}

/// Decodes the signed state that names `outpoint`.
///
/// A history with no genesis payload is `missing genesis`. The seal record is
/// not an input.
pub fn state_named_by_signed(
    payloads: &[&[u8]],
    outpoint: &[u8; 36],
) -> Result<ResultingState, &'static str> {
    let mut saw_genesis = false;
    let mut found = None;
    for payload in payloads {
        if payload.len() < 5 {
            return Err("truncated payload");
        }
        let object_type = u16::from_le_bytes([payload[3], payload[4]]);
        if !matches!(object_type, 1 | 2 | 3) {
            continue;
        }
        let state = decode_identity_state(payload)?;
        if object_type == 1 {
            saw_genesis = true;
        }
        if state.next_seal == *outpoint {
            if found.is_some() {
                return Err("more than one state names this seal");
            }
            found = Some(state);
        }
    }
    if !saw_genesis {
        return Err("missing genesis");
    }
    found.ok_or("no state names this seal")
}

/// Recomputes the seal script from the signed state that names `outpoint`.
pub fn script_for_named_seal(
    payloads: &[&[u8]],
    outpoint: &[u8; 36],
) -> Result<Vec<u8>, &'static str> {
    Ok(seal_for_state(&state_named_by_signed(payloads, outpoint)?)?.script_pubkey)
}

pub fn seal_for_state(state: &ResultingState) -> Result<SealScript, &'static str> {
    let policy = encode_seal_policy(&state.controller_bindings, &state.recovery_bindings);
    seal_script(
        &state.controller_bindings,
        &state.recovery_bindings,
        u64::from(state.recovery.threshold),
        u64::from(state.recovery.delay_blocks),
        policy,
    )
}

pub fn genesis(next_seal: [u8; 36]) -> SignedObject {
    let keys = demo_keys();
    genesis_with(keys.root, &demo_genesis_state(next_seal))
}

/// Frozen `official_name` claim. The subject is the history EntityID.
pub fn official_name_claim(
    network: u8,
    entity: [u8; 32],
    authorizing_state: [u8; 32],
    controller: DemoKey,
    name: &str,
    nonce: [u8; 32],
) -> Result<SignedObject, &'static str> {
    if name.is_empty() {
        return Err("official name is empty");
    }
    let mut payload = common_header(
        network,
        4,
        entity,
        Some(authorizing_state),
        key_id(1, controller.xonly),
        1,
        4,
    );
    payload.extend_from_slice(&entity);
    payload.extend(text_field("official_name")?);
    payload.extend(bytes_field(name.as_bytes()));
    payload.extend(option_fixed(None));
    payload.extend_from_slice(&nonce);
    payload.extend(option_fixed(None));
    payload.extend(option_fixed(None));
    Ok(sign(CLAIM_TAG, payload, controller))
}

/// Nonce for one live official-name claim. It is not a secret.
pub fn official_name_nonce(entity: &[u8; 32], name: &str) -> [u8; 32] {
    use bitcoin_hashes::{sha256, Hash};
    let mut preimage = entity.to_vec();
    preimage.extend_from_slice(name.as_bytes());
    sha256::Hash::hash(&preimage).to_byte_array()
}

pub fn controller_rotation_with(
    signer: DemoKey,
    history_entity: [u8; 32],
    prior_state: [u8; 32],
    state: &ResultingState,
) -> SignedObject {
    let mut payload = common_header(
        demo_network(),
        2,
        history_entity,
        Some(prior_state),
        key_id(1, signer.xonly),
        1,
        2,
    );
    payload.push(1);
    payload.extend_from_slice(&encode_resulting_state(state));
    sign(TRANSITION_TAG, payload, signer)
}

pub fn controller_rotation(
    prior_state: [u8; 32],
    previous_seal: [u8; 36],
    next_seal: [u8; 36],
) -> SignedObject {
    let keys = demo_keys();
    let genesis = genesis(previous_seal);
    controller_rotation_with(
        keys.controller_0,
        entity_id(&genesis.payload),
        prior_state,
        &demo_rotation_state(1, prior_state, previous_seal, next_seal),
    )
}

/// Signs one recovery payload per signer. Signers are emitted in key-id order.
pub fn recovery_authorizations(
    history_entity: [u8; 32],
    prior_state: [u8; 32],
    policy: &RecoveryPolicy,
    not_before_height: u32,
    state: &ResultingState,
    signers: &[DemoKey],
) -> Result<Vec<SignedObject>, &'static str> {
    if signers.is_empty() {
        return Err("recovery requires at least one signer");
    }
    let mut ordered = signers.to_vec();
    ordered.sort_by_key(|key| key_id(2, key.xonly));
    if ordered
        .windows(2)
        .any(|pair| pair[0].xonly == pair[1].xonly)
    {
        return Err("duplicate recovery signer");
    }
    let policy_hash = recovery_policy_hash(policy);
    let mut body = vec![3];
    body.extend_from_slice(&policy_hash);
    body.extend_from_slice(&not_before_height.to_le_bytes());
    body.extend_from_slice(&encode_resulting_state(state));
    Ok(ordered
        .into_iter()
        .map(|signer| {
            let mut payload = common_header(
                demo_network(),
                3,
                history_entity,
                Some(prior_state),
                key_id(2, signer.xonly),
                2,
                3,
            );
            payload.extend_from_slice(&body);
            sign(RECOVERY_TAG, payload, signer)
        })
        .collect())
}

pub fn signature_accepts(
    tag: &str,
    payload: &[u8],
    signature: [u8; 64],
    signer_xonly: [u8; 32],
) -> bool {
    let Ok(public) = XOnlyPublicKey::from_slice(&signer_xonly) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(&signature) else {
        return false;
    };
    Secp256k1::verification_only()
        .verify_schnorr(
            &signature,
            &Message::from_digest(tagged_hash(tag, payload)),
            &public,
        )
        .is_ok()
}

pub fn verify(object: &SignedObject) -> Result<(), &'static str> {
    if tagged_hash(object.tag, &object.payload) != object.digest {
        return Err("tagged digest mismatch");
    }
    let public =
        XOnlyPublicKey::from_slice(&object.signer_xonly).map_err(|_| "invalid x-only key")?;
    let signature = Signature::from_slice(&object.signature).map_err(|_| "invalid signature")?;
    Secp256k1::verification_only()
        .verify_schnorr(&signature, &Message::from_digest(object.digest), &public)
        .map_err(|_| "BIP340 verification failed")
}

/// Checks the demo controller-rotation envelope: role, capability, operation,
/// and a signature by the header's controller key.
pub fn verify_controller_rotation(object: &SignedObject) -> Result<(), &'static str> {
    verify(object)?;
    if object.tag != TRANSITION_TAG {
        return Err("controller-rotation authorization fields do not match the demo profile");
    }
    let (role, capability, operation) = header_role_capability_operation(&object.payload)?;
    // Operations 2 and 4 stay open. A signed payload must not pass through.
    match operation {
        Some(2) | Some(4) => return Err("unsupported in demo"),
        Some(3) => return Err("operation 3 is invalid in the identity-transition domain"),
        _ => {}
    }
    header_role_allowed(role)?;
    if role != 1 || capability != 2 || operation != Some(1) {
        return Err("controller-rotation authorization fields do not match the demo profile");
    }
    if key_id(1, object.signer_xonly) != signing_key_id(&object.payload)? {
        return Err("controller-rotation authorization fields do not match the demo profile");
    }
    Ok(())
}

fn signing_key_id(payload: &[u8]) -> Result<[u8; 32], &'static str> {
    // version u16, network, type u16, entity 32, option state.
    if payload.len() < 40 {
        return Err("truncated header");
    }
    let mut index = 2 + 1 + 2 + 32;
    let present = *payload.get(index).ok_or("truncated header")?;
    index += 1;
    if present == 1 {
        index += 32;
    } else if present != 0 {
        return Err("invalid authorizing-state option");
    }
    payload
        .get(index..index + 32)
        .ok_or("truncated signing key id")?
        .try_into()
        .map_err(|_| "truncated signing key id")
}

fn header_role_capability_operation(payload: &[u8]) -> Result<(u8, u16, Option<u8>), &'static str> {
    let mut index = 2 + 1 + 2 + 32;
    let present = *payload.get(index).ok_or("truncated header")?;
    index += 1;
    if present == 1 {
        index += 32;
    }
    index += 32;
    let role = *payload.get(index).ok_or("truncated header")?;
    index += 1;
    let capability = u16::from_le_bytes(
        payload
            .get(index..index + 2)
            .ok_or("truncated header")?
            .try_into()
            .map_err(|_| "truncated header")?,
    );
    index += 2;
    let object_type = u16::from_le_bytes(payload[3..5].try_into().map_err(|_| "truncated header")?);
    let operation = if object_type == 2 || object_type == 3 {
        Some(*payload.get(index).ok_or("truncated operation")?)
    } else {
        None
    };
    Ok((role, capability, operation))
}

/// Returns the successor outpoint committed by a v0.1 controller rotation.
pub fn transition_next_seal(object: &SignedObject) -> Result<[u8; 36], &'static str> {
    if object.tag != TRANSITION_TAG {
        return Err("not a complete v0.1 controller rotation payload");
    }
    let mut index = header_end(&object.payload)?;
    if object.payload.get(index) != Some(&1) {
        return Err("not a controller-rotation operation");
    }
    index += 1;
    index += 8;
    index = skip_option(&object.payload, index, 32)?;
    index = skip_option(&object.payload, index, 36)?;
    object
        .payload
        .get(index..index + 36)
        .ok_or("invalid successor seal length")?
        .try_into()
        .map_err(|_| "invalid successor seal length")
}

fn header_end(payload: &[u8]) -> Result<usize, &'static str> {
    let mut index = 2 + 1 + 2 + 32;
    let present = *payload.get(index).ok_or("truncated header")?;
    index += 1;
    if present == 1 {
        index += 32;
    }
    index += 32 + 1 + 2;
    Ok(index)
}

fn skip_option(payload: &[u8], index: usize, fixed: usize) -> Result<usize, &'static str> {
    match payload.get(index) {
        Some(0) => Ok(index + 1),
        Some(1) => Ok(index + 1 + fixed),
        _ => Err("invalid option"),
    }
}

#[cfg(test)]
mod conformance;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_is_pinned() {
        assert_eq!(SPEC_COMMIT.len(), 40);
        assert_eq!(CANONICAL_RULES.len(), 4);
    }

    #[test]
    fn explicit_coin_and_network_match_the_env_helpers() {
        let keys = demo_keys();
        assert_eq!(keys, keys_for(demo_coin()));
        let next = [9u8; 36];
        let state = demo_genesis_state(next);
        let encoded = encode_resulting_state(&state);
        assert_eq!(
            encoded,
            encode_resulting_state(&genesis_state_from(&keys, next))
        );
        let signed = genesis_with(keys.root, &state);
        assert_eq!(signed, genesis_for(demo_network(), keys.root, &state));
    }

    fn fresh_key(fill: u8) -> DemoKey {
        let secret = [fill; 32];
        let parsed = SecretKey::from_slice(&secret).expect("scalar");
        let pair = Keypair::from_secret_key(&Secp256k1::new(), &parsed);
        let (xonly, _) = pair.x_only_public_key();
        DemoKey {
            secret,
            xonly: xonly.serialize(),
        }
    }

    fn fresh_state(next_seal: [u8; 36], seal_byte: u8) -> ResultingState {
        let controller = fresh_key(0x21);
        ResultingState {
            sequence: 0,
            previous_state: None,
            previous_seal: None,
            next_seal,
            controllers: vec![ControllerEntry {
                xonly: controller.xonly,
                capabilities: vec![2, 4],
            }],
            recovery: RecoveryPolicy {
                version: 1,
                sequence: 0,
                threshold: 1,
                key_ids: vec![key_id(2, fresh_key(0x22).xonly)],
                delay_blocks: 1008,
                cancellation_rule: 1,
            },
            controller_bindings: vec![SealBinding {
                authorizing_key_id: key_id(1, controller.xonly),
                seal_xonly: [seal_byte; 32],
            }],
            recovery_bindings: vec![SealBinding {
                authorizing_key_id: key_id(2, fresh_key(0x23).xonly),
                seal_xonly: [seal_byte.wrapping_add(1); 32],
            }],
            lifecycle_status: 1,
        }
    }

    fn lineage(expected: Vec<u8>, observed: Vec<u8>) -> LineageEvidence {
        let txid = [0x44u8; 32];
        let mut header = [0u8; 80];
        header[36..68].copy_from_slice(&txid);
        LineageEvidence {
            seals: vec![SealFact {
                expected_script: expected,
                observed_script: observed,
                creation: InclusionProof {
                    txid,
                    index: 0,
                    siblings: Vec::new(),
                    header,
                    height: 100,
                },
            }],
            anchor: None,
            observation: Some(CurrentSealView {
                unspent: true,
                spend: None,
            }),
            o2a_ok: true,
            valid_transition: false,
            best_height: 105,
            required_depth: 1,
        }
    }

    #[test]
    fn identity_state_roundtrip_rejects_truncated_and_trailing_bytes() {
        let state = fresh_state([0x41; 36], 0x55);
        let payload = genesis_with(fresh_key(0x11), &state).payload;
        assert_eq!(decode_identity_state(&payload).unwrap(), state);
        decode_payload(&payload).unwrap();
        assert_eq!(
            decode_identity_state(&payload[..payload.len() - 1]),
            Err("truncated payload")
        );
        let mut extra = payload.clone();
        extra.push(0);
        assert_eq!(decode_identity_state(&extra), Err("trailing payload bytes"));
    }

    #[test]
    fn record_script_matching_the_chain_but_not_the_policy_is_invalid() {
        let outpoint = [0x41; 36];
        let state = fresh_state(outpoint, 0x55);
        let payload = genesis_with(fresh_key(0x11), &state).payload;
        let policy_script = script_for_named_seal(&[&payload], &outpoint).unwrap();
        let record_script = b"record-script-that-matches-the-chain".to_vec();
        assert_ne!(record_script, policy_script);
        let report = evaluate_lineage(&lineage(policy_script, record_script), "imported");
        assert_eq!(report.identity_history_state, "INVALID");
        assert_eq!(report.bitcoin, "script does not match the policy");
    }

    #[test]
    fn policy_bytes_outside_the_seal_policy_field_do_not_select_the_script() {
        let outpoint = [0x42; 36];
        let state = fresh_state(outpoint, 0x66);
        let payload = genesis_with(fresh_key(0x12), &state).payload;
        let policy = encode_seal_policy(&state.controller_bindings, &state.recovery_bindings);
        let at = payload
            .windows(policy.len())
            .position(|window| window == policy.as_slice())
            .expect("seal policy field");
        let stray = if at == 0 {
            &payload[1..1 + policy.len()]
        } else {
            &payload[..policy.len()]
        };
        assert_ne!(stray, policy.as_slice());
        assert!(payload.windows(stray.len()).any(|window| window == stray));
        let selected = script_for_named_seal(&[&payload], &outpoint).unwrap();
        let policy_script = seal_for_state(&state).unwrap().script_pubkey;
        assert_eq!(selected, policy_script);
        let report = evaluate_lineage(
            &lineage(selected, b"script-from-the-stray-bytes".to_vec()),
            "imported",
        );
        assert_eq!(report.identity_history_state, "INVALID");
    }

    #[test]
    fn history_without_a_genesis_is_never_current() {
        let outpoint = [0x43; 36];
        assert_eq!(
            script_for_named_seal(&[], &outpoint),
            Err("missing genesis")
        );
        let state = fresh_state(outpoint, 0x77);
        let transition = {
            let mut payload = common_header(
                4,
                2,
                [0x9; 32],
                Some([0x8; 32]),
                key_id(1, fresh_key(0x21).xonly),
                1,
                2,
            );
            payload.push(1);
            payload.extend(encode_resulting_state(&state));
            payload
        };
        assert_eq!(
            script_for_named_seal(&[&transition], &outpoint),
            Err("missing genesis")
        );
    }

    #[test]
    fn signed_state_is_the_decoded_object() {
        let outpoint = [0x45; 36];
        let state = fresh_state(outpoint, 0x91);
        let payload = genesis_with(fresh_key(0x14), &state).payload;
        let decoded = state_named_by_signed(&[&payload], &outpoint).unwrap();
        assert_eq!(decoded, state);
        let script = seal_for_state(&decoded).unwrap().script_pubkey;
        let demo_script = seal_for_state(&demo_genesis_state(outpoint))
            .unwrap()
            .script_pubkey;
        let hostile = b"hostile-record-script".to_vec();
        assert_ne!(script, demo_script);
        assert_ne!(script, hostile);
        assert_eq!(
            script_for_named_seal(&[&payload], &outpoint).unwrap(),
            script
        );
        assert_eq!(
            state_named_by_signed(&[], &outpoint),
            Err("missing genesis")
        );
    }

    #[test]
    fn fresh_seed_genesis_verifies_without_demo_keys() {
        let outpoint = [0x44; 36];
        let state = fresh_state(outpoint, 0x88);
        let payload = genesis_with(fresh_key(0x13), &state).payload;
        let script = script_for_named_seal(&[&payload], &outpoint).unwrap();
        let demo_script = seal_for_state(&demo_genesis_state(outpoint))
            .unwrap()
            .script_pubkey;
        assert_ne!(script, demo_script);
        let report = evaluate_lineage(&lineage(script.clone(), script), "imported");
        assert_eq!(report.identity_history_state, "CURRENT");
    }

    #[test]
    fn published_demo_keys_match_the_regtest_vectors() {
        let keys = demo_keys();
        assert_eq!(
            hex::encode(keys.root.xonly),
            "f53704a3e4d3ddad5efb561617489c31522a0cf5a3098f5feb15f900b5ded236"
        );
        assert_eq!(
            hex::encode(keys.controller_0.xonly),
            "ff96950266bcbdc5f9131f722305116cd5e51fe9c1d9e2c1073e4fcb4618c3c6"
        );
    }

    #[test]
    fn signed_objects_verify_and_are_domain_separated() {
        let genesis = genesis([1; 36]);
        verify(&genesis).expect("genesis signature");
        let transition = controller_rotation([2; 32], [1; 36], [3; 36]);
        verify_controller_rotation(&transition).expect("transition authorization");
        assert_eq!(transition_next_seal(&transition), Ok([3; 36]));
        assert_ne!(genesis.digest, transition.digest);
        let recovery = recovery_authorizations(
            entity_id(&genesis.payload),
            [2; 32],
            &demo_recovery_policy(),
            112,
            &demo_rotation_state(2, [2; 32], [3; 36], [4; 36]),
            &[demo_keys().recovery_0, demo_keys().recovery_2],
        )
        .expect("recovery signatures");
        assert_eq!(recovery.len(), 2);
        verify(&recovery[0]).expect("first recovery signature");
        assert_ne!(recovery[0].digest, recovery[1].digest);
    }

    #[test]
    fn valid_signature_cannot_replace_controller_capability() {
        let mut payload = controller_rotation([2; 32], [1; 36], [3; 36]).payload;
        payload[103..105].copy_from_slice(&1u16.to_le_bytes());
        let wrongly_authorized = sign(TRANSITION_TAG, payload, demo_keys().controller_0);
        verify(&wrongly_authorized).expect("signature remains cryptographically valid");
        assert_eq!(
            verify_controller_rotation(&wrongly_authorized),
            Err("controller-rotation authorization fields do not match the demo profile")
        );
    }

    #[test]
    fn policy_change_and_custody_transfer_are_unsupported() {
        let base = controller_rotation([2; 32], [1; 36], [3; 36]);
        let end = header_end(&base.payload).expect("header");
        assert_eq!(base.payload[end], 1);
        for operation in [2u8, 4] {
            let mut payload = base.payload.clone();
            payload[end] = operation;
            let signed = sign(TRANSITION_TAG, payload, demo_keys().controller_0);
            verify(&signed).expect("the mutated operation is still signed");
            assert_eq!(
                verify_controller_rotation(&signed),
                Err("unsupported in demo")
            );
        }
        let mut payload = base.payload.clone();
        payload[end] = 3;
        let signed = sign(TRANSITION_TAG, payload, demo_keys().controller_0);
        assert_eq!(
            verify_controller_rotation(&signed),
            Err("operation 3 is invalid in the identity-transition domain")
        );
    }

    #[test]
    fn seal_role_and_stale_binding_are_rejected() {
        assert_eq!(
            header_role_allowed(4),
            Err("seal role rejected in signed headers")
        );
        let keys = demo_keys();
        let current = [key_id(1, keys.controller_1.xonly)];
        let stale = [binding(
            1,
            keys.controller_0.xonly,
            keys.seal_controller_0.xonly,
        )];
        let recovery_ids = demo_recovery_policy().key_ids;
        let recovery = demo_genesis_state([0; 36]).recovery_bindings;
        assert_eq!(
            seal_bindings_valid(&stale, &recovery, &current, &recovery_ids, &[]),
            Err("controller seal bindings do not cover the transition key set")
        );
        let mut reused = recovery.clone();
        reused[0].seal_xonly = keys.controller_0.xonly;
        assert_eq!(
            seal_bindings_valid(
                &demo_genesis_state([0; 36]).controller_bindings,
                &reused,
                &[key_id(1, keys.controller_0.xonly)],
                &recovery_ids,
                &[keys.controller_0.xonly],
            ),
            Err("cross-role x-only reuse")
        );
    }

    #[test]
    fn official_name_roundtrips_from_the_signed_claim() {
        let keys = demo_keys();
        let state = genesis_state_from(&keys, [7u8; 36]);
        let genesis = genesis_for(NETWORK_REGTEST, keys.root, &state);
        let entity = entity_id(&genesis.payload);
        let decoded = decode_identity_state(&genesis.payload).expect("genesis state");
        let sid = state_id(&entity, &encode_resulting_state(&decoded));
        let name = "Rehearsal Name";
        let claim = official_name_claim(
            NETWORK_REGTEST,
            entity,
            sid,
            keys.controller_0,
            name,
            official_name_nonce(&entity, name),
        )
        .expect("claim");
        let parsed = decode::parse_claim(&claim.payload).expect("parse");
        assert_eq!(parsed.predicate, "official_name");
        assert_eq!(parsed.object, name.as_bytes());
        assert_eq!(official_name_of(&claim.payload).expect("name"), name);
    }
}
