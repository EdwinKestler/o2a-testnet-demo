//! Canonical O2A-CANON-1 byte layout at spec 0ef16c2.
//!
//! Lists are a little-endian `u32` count followed by the items. Options are
//! `0x00`, or `0x01` plus the fixed value.

use crate::tagged_hash;

pub fn u16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

pub fn u32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

pub fn u64(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

pub fn option_fixed(value: Option<&[u8]>) -> Vec<u8> {
    match value {
        None => vec![0],
        Some(value) => {
            let mut out = Vec::with_capacity(1 + value.len());
            out.push(1);
            out.extend_from_slice(value);
            out
        }
    }
}

pub fn list_items(items: &[impl AsRef<[u8]>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u32(items.len() as u32));
    for item in items {
        out.extend_from_slice(item.as_ref());
    }
    out
}

pub fn bytes_field(value: &[u8]) -> Vec<u8> {
    let mut out = u32(value.len() as u32).to_vec();
    out.extend_from_slice(value);
    out
}

pub fn text_field(value: &str) -> Result<Vec<u8>, &'static str> {
    let encoded = value.as_bytes();
    if encoded.is_empty() || encoded.contains(&0) || encoded.len() > 4096 {
        return Err("invalid text field");
    }
    Ok(bytes_field(encoded))
}

pub fn key_id(role: u8, xonly: [u8; 32]) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(33);
    preimage.push(role);
    preimage.extend_from_slice(&xonly);
    tagged_hash("O2A/v0.1/key-id", &preimage)
}

pub fn entity_id(genesis_payload: &[u8]) -> [u8; 32] {
    tagged_hash("O2A/v0.1/entity-id", genesis_payload)
}

/// Signer-independent id of one resulting state for one EntityID.
pub fn state_id(entity: &[u8; 32], resulting_state: &[u8]) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(32 + resulting_state.len());
    preimage.extend_from_slice(entity);
    preimage.extend_from_slice(resulting_state);
    tagged_hash("O2A/v0.1/state-id", &preimage)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryPolicy {
    pub version: u16,
    pub sequence: u64,
    pub threshold: u16,
    pub key_ids: Vec<[u8; 32]>,
    pub delay_blocks: u32,
    pub cancellation_rule: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SealBinding {
    pub authorizing_key_id: [u8; 32],
    pub seal_xonly: [u8; 32],
}

impl SealBinding {
    pub fn bytes(self) -> [u8; 64] {
        let mut out = [0u8; 64];
        out[..32].copy_from_slice(&self.authorizing_key_id);
        out[32..].copy_from_slice(&self.seal_xonly);
        out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerEntry {
    pub xonly: [u8; 32],
    pub capabilities: Vec<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultingState {
    pub sequence: u64,
    pub previous_state: Option<[u8; 32]>,
    pub previous_seal: Option<[u8; 36]>,
    pub next_seal: [u8; 36],
    pub controllers: Vec<ControllerEntry>,
    pub recovery: RecoveryPolicy,
    pub controller_bindings: Vec<SealBinding>,
    pub recovery_bindings: Vec<SealBinding>,
    pub lifecycle_status: u8,
}

pub fn encode_controller(entry: &ControllerEntry) -> Vec<u8> {
    let mut capabilities = entry.capabilities.clone();
    capabilities.sort_unstable();
    let mut out = Vec::new();
    out.extend_from_slice(&key_id(1, entry.xonly));
    out.extend_from_slice(&entry.xonly);
    out.push(1);
    out.extend_from_slice(&list_items(
        &capabilities
            .iter()
            .map(|capability| u16(*capability).to_vec())
            .collect::<Vec<_>>(),
    ));
    out
}

pub fn encode_recovery_policy(policy: &RecoveryPolicy) -> Vec<u8> {
    let mut key_ids = policy.key_ids.clone();
    key_ids.sort();
    let mut out = Vec::new();
    out.extend_from_slice(&u16(policy.version));
    out.extend_from_slice(&u64(policy.sequence));
    out.extend_from_slice(&u16(policy.threshold));
    out.extend_from_slice(&list_items(
        &key_ids.iter().map(|id| id.to_vec()).collect::<Vec<_>>(),
    ));
    out.extend_from_slice(&u32(policy.delay_blocks));
    out.push(policy.cancellation_rule);
    out
}

pub fn encode_seal_policy(
    controller_bindings: &[SealBinding],
    recovery_bindings: &[SealBinding],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u16(1));
    out.extend_from_slice(&list_items(
        &controller_bindings
            .iter()
            .map(|binding| binding.bytes().to_vec())
            .collect::<Vec<_>>(),
    ));
    out.extend_from_slice(&list_items(
        &recovery_bindings
            .iter()
            .map(|binding| binding.bytes().to_vec())
            .collect::<Vec<_>>(),
    ));
    out
}

pub fn encode_resulting_state(state: &ResultingState) -> Vec<u8> {
    let mut controllers = state.controllers.clone();
    controllers.sort_by_key(|controller| key_id(1, controller.xonly));
    let controller_bytes = controllers
        .iter()
        .map(encode_controller)
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    out.extend_from_slice(&u64(state.sequence));
    out.extend_from_slice(&option_fixed(
        state.previous_state.as_ref().map(|v| v.as_slice()),
    ));
    out.extend_from_slice(&option_fixed(
        state.previous_seal.as_ref().map(|v| v.as_slice()),
    ));
    out.extend_from_slice(&state.next_seal);
    out.extend_from_slice(&list_items(&controller_bytes));
    out.extend_from_slice(&encode_recovery_policy(&state.recovery));
    out.extend_from_slice(&encode_seal_policy(
        &state.controller_bindings,
        &state.recovery_bindings,
    ));
    out.extend_from_slice(&option_fixed(None));
    out.push(state.lifecycle_status);
    out.extend_from_slice(&option_fixed(None));
    out.extend_from_slice(&option_fixed(None));
    out
}

pub fn common_header(
    network: u8,
    object_type: u16,
    signer_entity: [u8; 32],
    authorizing_state: Option<[u8; 32]>,
    signing_key_id: [u8; 32],
    role: u8,
    capability: u16,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u16(1));
    out.push(network);
    out.extend_from_slice(&u16(object_type));
    out.extend_from_slice(&signer_entity);
    out.extend_from_slice(&option_fixed(
        authorizing_state.as_ref().map(|value| value.as_slice()),
    ));
    out.extend_from_slice(&signing_key_id);
    out.push(role);
    out.extend_from_slice(&u16(capability));
    out
}

pub fn content_reference(
    media_type: &str,
    length: u64,
    digest: [u8; 32],
) -> Result<Vec<u8>, &'static str> {
    let mut out = text_field(media_type)?;
    out.extend_from_slice(&u64(length));
    out.extend_from_slice(&digest);
    Ok(out)
}

/// Standard demo controller capabilities, strictly increasing.
pub fn demo_controller_capabilities() -> Vec<u16> {
    vec![2, 4, 5, 6, 7, 8, 9, 10, 11, 12]
}
