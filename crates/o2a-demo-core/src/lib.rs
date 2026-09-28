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
pub use decode::{decode_payload, evaluate_name_claim, ClaimAuthorization};
pub use encode::{
    bytes_field, common_header, content_reference, encode_recovery_policy, encode_resulting_state,
    encode_seal_policy, entity_id as entity_id_of_payload, key_id, list_items, option_fixed,
    text_field, ControllerEntry, RecoveryPolicy, ResultingState, SealBinding,
};
pub use eval::{
    adapter_key_distinct, adapter_scheme, capability_known, evidence_ids_valid, genesis_root_ok,
    header_role_allowed, identity_history_state, increasing_expiry, manifest_binding,
    observation_time_status, package_object_gap, recovery_policy_valid, recovery_witness_status,
    revocation_target, seal_bindings_valid, seal_output_matches, seal_policy_valid,
    state_authorizes, HistoryInput, RecoveryClock, SealWatch,
};
pub use seal::{recovery_leaf, script_num, seal_script, SealScript, NUMS_X};

pub const SPEC_COMMIT: &str = "0ef16c2132ea54cdfd4aa86a34a748f998f388d8";
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoKey {
    secret: [u8; 32],
    pub xonly: [u8; 32],
}

impl DemoKey {
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
    let seed = unsafe_seed();
    let entity = demo_entity_index();
    let key = |role: u32, index: u32| derive::identity_key(&seed, 1, entity, role, index);
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

pub fn demo_recovery_policy() -> RecoveryPolicy {
    let keys = demo_keys();
    let mut key_ids = [
        key_id(2, keys.recovery_0.xonly),
        key_id(2, keys.recovery_1.xonly),
        key_id(2, keys.recovery_2.xonly),
    ];
    key_ids.sort();
    RecoveryPolicy {
        version: 1,
        sequence: 1,
        threshold: DEMO_RECOVERY_THRESHOLD,
        key_ids: key_ids.to_vec(),
        delay_blocks: DEMO_DELAY_BLOCKS,
        cancellation_rule: 1,
    }
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
    let keys = demo_keys();
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
        recovery: demo_recovery_policy(),
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
    let mut payload = common_header(
        NETWORK_REGTEST,
        1,
        [0u8; 32],
        None,
        key_id(0, root.xonly),
        0,
        1,
    );
    payload.extend_from_slice(&2u16.to_le_bytes());
    payload.extend_from_slice(&root.xonly);
    payload.extend_from_slice(&encode_resulting_state(state));
    sign(GENESIS_TAG, payload, root)
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

pub fn controller_rotation_with(
    signer: DemoKey,
    history_entity: [u8; 32],
    prior_state: [u8; 32],
    state: &ResultingState,
) -> SignedObject {
    let mut payload = common_header(
        NETWORK_REGTEST,
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
                NETWORK_REGTEST,
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
}
