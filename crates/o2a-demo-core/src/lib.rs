//! Deterministic, network-free O2A encoding and verification boundary.
//!
//! This is the demo's single implementation of the normative byte and signing
//! rules referenced by [`CANONICAL_RULES`]. It accepts explicit state and chain
//! evidence; it never reads the network, clock, or a database.

use bitcoin_hashes::{
    hmac::{Hmac, HmacEngine},
    sha256, sha512, Hash, HashEngine,
};
use secp256k1::{
    schnorr::Signature, Keypair, Message, Scalar, Secp256k1, SecretKey, XOnlyPublicKey,
};

pub const SPEC_COMMIT: &str = "3ca98ea9b60256f271e71fa89caa09448e804e87";
pub const CANONICAL_RULES: [&str; 2] = [
    "../o2a-protocol/specs/canonical-encoding.md",
    "../o2a-protocol/specs/cryptographic-profile.md",
];
pub const UNSAFE_BIP39_SEED_HEX: &str = concat!(
    "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e5349553",
    "1f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04"
);
pub const NETWORK_REGTEST: u8 = 4;

const HARDENED: u32 = 1 << 31;
const O2A_INDEX: u32 = 998_536_622;
const ENTITY_TAG: &str = "O2A/v0.1/entity-id";
const KEY_TAG: &str = "O2A/v0.1/key-id";
const GENESIS_TAG: &str = "O2A/v0.1/entity-genesis";
const TRANSITION_TAG: &str = "O2A/v0.1/identity-transition";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Xprv {
    secret: [u8; 32],
    chain_code: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoKey {
    secret: [u8; 32],
    pub xonly: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoKeys {
    pub root: DemoKey,
    pub controller_0: DemoKey,
    pub controller_1: DemoKey,
    pub recovery_0: DemoKey,
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

fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    let mut engine = HmacEngine::<sha512::Hash>::new(key);
    engine.input(data);
    Hmac::<sha512::Hash>::from_engine(engine).to_byte_array()
}

fn master(seed: &[u8]) -> Xprv {
    let digest = hmac_sha512(b"Bitcoin seed", seed);
    Xprv {
        secret: digest[..32].try_into().expect("fixed hash length"),
        chain_code: digest[32..].try_into().expect("fixed hash length"),
    }
}

fn child(parent: Xprv, child_number: u32) -> Xprv {
    let mut data = [0u8; 37];
    if child_number >= HARDENED {
        data[1..33].copy_from_slice(&parent.secret);
    } else {
        let key = SecretKey::from_slice(&parent.secret).expect("valid parent key");
        data[..33].copy_from_slice(
            &secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &key).serialize(),
        );
    }
    data[33..].copy_from_slice(&child_number.to_be_bytes());
    let digest = hmac_sha512(&parent.chain_code, &data);
    let tweak = Scalar::from_be_bytes(digest[..32].try_into().expect("fixed hash length"))
        .expect("demo derivation tweak must be in range");
    let parent = SecretKey::from_slice(&parent.secret).expect("valid parent key");
    let derived = parent
        .add_tweak(&tweak)
        .expect("demo child key must be nonzero");
    Xprv {
        secret: derived.secret_bytes(),
        chain_code: digest[32..].try_into().expect("fixed hash length"),
    }
}

fn hard(index: u32) -> u32 {
    assert!(index < HARDENED, "demo index must fit in 31 bits");
    index | HARDENED
}

fn derive(mut key: Xprv, path: &[u32]) -> Xprv {
    for index in path {
        key = child(key, *index);
    }
    key
}

fn bip85_o2a(master: Xprv) -> Xprv {
    let key = derive(master, &[hard(83_696_968), hard(32), hard(O2A_INDEX)]);
    let digest = hmac_sha512(b"bip-entropy-from-k", &key.secret);
    Xprv {
        chain_code: digest[..32].try_into().expect("fixed hash length"),
        secret: digest[32..].try_into().expect("fixed hash length"),
    }
}

fn demo_key(xprv: Xprv) -> DemoKey {
    let secret = SecretKey::from_slice(&xprv.secret).expect("valid derived key");
    let pair = Keypair::from_secret_key(&Secp256k1::new(), &secret);
    let (xonly, _) = XOnlyPublicKey::from_keypair(&pair);
    DemoKey {
        secret: xprv.secret,
        xonly: xonly.serialize(),
    }
}

/// Derives the disposable regtest entity 0 keys from the published unsafe seed.
pub fn demo_keys() -> DemoKeys {
    let seed = hex::decode(UNSAFE_BIP39_SEED_HEX).expect("published seed hex");
    let o2a = bip85_o2a(master(&seed));
    let key =
        |role: u32, index: u32| demo_key(derive(o2a, &[hard(1), hard(0), hard(role), hard(index)]));
    DemoKeys {
        root: key(0, 0),
        controller_0: key(1, 0),
        controller_1: key(1, 1),
        recovery_0: key(2, 0),
    }
}

pub fn tagged_hash(tag: &str, payload: &[u8]) -> [u8; 32] {
    let tag_hash = sha256::Hash::hash(tag.as_bytes()).to_byte_array();
    let mut engine = sha256::Hash::engine();
    engine.input(&tag_hash);
    engine.input(&tag_hash);
    engine.input(payload);
    sha256::Hash::from_engine(engine).to_byte_array()
}

pub fn entity_id(root_xonly: [u8; 32]) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(35);
    preimage.extend_from_slice(&1u16.to_le_bytes());
    preimage.push(NETWORK_REGTEST);
    preimage.extend_from_slice(&root_xonly);
    tagged_hash(ENTITY_TAG, &preimage)
}

pub fn key_id(role: u8, xonly: [u8; 32]) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(33);
    preimage.push(role);
    preimage.extend_from_slice(&xonly);
    tagged_hash(KEY_TAG, &preimage)
}

pub fn recovery_policy_hash(recovery_xonly: [u8; 32]) -> [u8; 32] {
    let mut policy = Vec::new();
    recovery_policy(&mut policy, recovery_xonly);
    tagged_hash("O2A/v0.1/recovery-policy", &policy)
}

fn option_fixed(out: &mut Vec<u8>, value: Option<&[u8]>) {
    match value {
        None => out.push(0),
        Some(value) => {
            out.push(1);
            out.extend_from_slice(value);
        }
    }
}

fn controller(out: &mut Vec<u8>, xonly: [u8; 32]) {
    out.extend_from_slice(&key_id(1, xonly));
    out.extend_from_slice(&xonly);
    out.push(1);
    const CAPS: [u16; 10] = [2, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    out.extend_from_slice(&(CAPS.len() as u32).to_le_bytes());
    for cap in CAPS {
        out.extend_from_slice(&cap.to_le_bytes());
    }
}

fn recovery_policy(out: &mut Vec<u8>, recovery_xonly: [u8; 32]) {
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u64.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&key_id(2, recovery_xonly));
    out.extend_from_slice(&6u32.to_le_bytes());
    out.push(1);
}

fn resulting_state(
    sequence: u64,
    previous_state: Option<[u8; 32]>,
    previous_seal: Option<[u8; 36]>,
    next_seal: [u8; 36],
    controller_xonly: [u8; 32],
    recovery_xonly: [u8; 32],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&sequence.to_le_bytes());
    option_fixed(&mut out, previous_state.as_ref().map(<[u8; 32]>::as_slice));
    option_fixed(&mut out, previous_seal.as_ref().map(<[u8; 36]>::as_slice));
    out.extend_from_slice(&next_seal);
    out.extend_from_slice(&1u32.to_le_bytes());
    controller(&mut out, controller_xonly);
    recovery_policy(&mut out, recovery_xonly);
    out.push(0);
    out.push(1);
    out.push(0);
    out.push(0);
    out
}

fn header(
    object_type: u16,
    signer_entity: [u8; 32],
    authorizing_state: Option<[u8; 32]>,
    signing_key_id: [u8; 32],
    role: u8,
    capability: u16,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.push(NETWORK_REGTEST);
    out.extend_from_slice(&object_type.to_le_bytes());
    out.extend_from_slice(&signer_entity);
    option_fixed(
        &mut out,
        authorizing_state.as_ref().map(<[u8; 32]>::as_slice),
    );
    out.extend_from_slice(&signing_key_id);
    out.push(role);
    out.extend_from_slice(&capability.to_le_bytes());
    out
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

pub fn genesis(next_seal: [u8; 36]) -> SignedObject {
    let keys = demo_keys();
    let entity = entity_id(keys.root.xonly);
    let mut payload = header(1, entity, None, key_id(0, keys.root.xonly), 0, 1);
    payload.extend_from_slice(&2u16.to_le_bytes());
    payload.extend_from_slice(&keys.root.xonly);
    payload.extend_from_slice(&resulting_state(
        0,
        None,
        None,
        next_seal,
        keys.controller_0.xonly,
        keys.recovery_0.xonly,
    ));
    sign(GENESIS_TAG, payload, keys.root)
}

pub fn controller_rotation(
    prior_state: [u8; 32],
    previous_seal: [u8; 36],
    next_seal: [u8; 36],
) -> SignedObject {
    let keys = demo_keys();
    let entity = entity_id(keys.root.xonly);
    let mut payload = header(
        2,
        entity,
        Some(prior_state),
        key_id(1, keys.controller_0.xonly),
        1,
        2,
    );
    payload.push(1);
    payload.extend_from_slice(&resulting_state(
        1,
        Some(prior_state),
        Some(previous_seal),
        next_seal,
        keys.controller_1.xonly,
        keys.recovery_0.xonly,
    ));
    sign(TRANSITION_TAG, payload, keys.controller_0)
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

/// Verifies the exact demo-stable v0.1 controller-rotation envelope.
///
/// This is intentionally stricter than signature verification: it rejects a
/// correctly signed object if the network, object type, entity, authorizing
/// state, key role, capability, or operation is not the controller-rotation
/// profile used by this evidence run.
pub fn verify_controller_rotation(object: &SignedObject) -> Result<(), &'static str> {
    verify(object)?;
    if object.tag != TRANSITION_TAG || object.payload.len() != 370 {
        return Err("not an exact v0.1 controller rotation payload");
    }
    let keys = demo_keys();
    let entity = entity_id(keys.root.xonly);
    if object.payload[0..2] != 1u16.to_le_bytes()
        || object.payload[2] != NETWORK_REGTEST
        || object.payload[3..5] != 2u16.to_le_bytes()
        || object.payload[5..37] != entity
        || object.payload[37] != 1
        || object.payload[70..102] != key_id(1, keys.controller_0.xonly)
        || object.payload[102] != 1
        || object.payload[103..105] != 2u16.to_le_bytes()
        || object.payload[105] != 1
        || object.signer_xonly != keys.controller_0.xonly
    {
        return Err("controller-rotation authorization fields do not match the demo profile");
    }
    if object.payload[38..70] != object.payload[115..147] {
        return Err("authorizing state and previous state differ");
    }
    if object.payload[114] != 1 || object.payload[147] != 1 {
        return Err("controller rotation omits required prior state or seal");
    }
    Ok(())
}

/// Returns the exact successor outpoint committed by a v0.1 controller rotation.
pub fn transition_next_seal(object: &SignedObject) -> Result<[u8; 36], &'static str> {
    if object.tag != TRANSITION_TAG || object.payload.len() < 220 {
        return Err("not a complete v0.1 controller rotation payload");
    }
    if object.payload[105] != 1 {
        return Err("not a controller-rotation operation");
    }
    object.payload[184..220]
        .try_into()
        .map_err(|_| "invalid successor seal length")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_is_pinned() {
        assert_eq!(SPEC_COMMIT.len(), 40);
        assert_eq!(CANONICAL_RULES.len(), 2);
    }

    #[test]
    fn published_demo_keys_match_the_spec_vectors() {
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
}
