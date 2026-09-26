//! Local authorization checks. These do not read a network or a clock.

use crate::encode::{RecoveryPolicy, SealBinding};

pub fn recovery_policy_valid(policy: &RecoveryPolicy) -> Result<(), &'static str> {
    if policy.version != 1 {
        return Err("unknown recovery-policy version");
    }
    if policy.key_ids.is_empty() || policy.key_ids.len() > 16 {
        return Err("recovery key set must contain 1 to 16 keys");
    }
    if policy.threshold == 0 || policy.threshold as usize > policy.key_ids.len() {
        return Err("recovery threshold is outside the key set");
    }
    if !(1..=65_535).contains(&policy.delay_blocks) {
        return Err("delay_blocks is outside 1..=65535");
    }
    if policy.cancellation_rule != 1 {
        return Err("unknown cancellation rule");
    }
    let mut sorted = policy.key_ids.clone();
    sorted.sort();
    if sorted != policy.key_ids || sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("recovery key ids must be strictly sorted");
    }
    Ok(())
}

pub fn seal_policy_valid(
    version: u16,
    controller_bindings: &[SealBinding],
    recovery_bindings: &[SealBinding],
    transition_controller_ids: &[[u8; 32]],
    recovery_key_ids: &[[u8; 32]],
    forbidden_xonly: &[[u8; 32]],
) -> Result<(), &'static str> {
    if version != 1 {
        return Err("unknown seal-policy version");
    }
    seal_bindings_valid(
        controller_bindings,
        recovery_bindings,
        transition_controller_ids,
        recovery_key_ids,
        forbidden_xonly,
    )
}

pub fn seal_bindings_valid(
    controller_bindings: &[SealBinding],
    recovery_bindings: &[SealBinding],
    transition_controller_ids: &[[u8; 32]],
    recovery_key_ids: &[[u8; 32]],
    forbidden_xonly: &[[u8; 32]],
) -> Result<(), &'static str> {
    if controller_bindings.is_empty() || controller_bindings.len() > 16 {
        return Err("controller seal bindings must contain 1 to 16 entries");
    }
    if recovery_bindings.is_empty() || recovery_bindings.len() > 16 {
        return Err("recovery seal bindings must contain 1 to 16 entries");
    }
    if !strictly_sorted(
        &controller_bindings
            .iter()
            .map(|binding| binding.bytes())
            .collect::<Vec<_>>(),
    ) {
        return Err("unsorted controller seal bindings");
    }
    if !strictly_sorted(
        &recovery_bindings
            .iter()
            .map(|binding| binding.bytes())
            .collect::<Vec<_>>(),
    ) {
        return Err("unsorted recovery seal bindings");
    }
    let controller_ids = controller_bindings
        .iter()
        .map(|binding| binding.authorizing_key_id)
        .collect::<Vec<_>>();
    let recovery_ids = recovery_bindings
        .iter()
        .map(|binding| binding.authorizing_key_id)
        .collect::<Vec<_>>();
    if !unique(&controller_ids) || !unique(&recovery_ids) {
        return Err("duplicate authorizing key in seal bindings");
    }
    let mut all_ids = controller_ids.clone();
    all_ids.extend(recovery_ids.iter().copied());
    if !unique(&all_ids) {
        return Err("authorizing key reused across seal binding lists");
    }
    if !same_set(&controller_ids, transition_controller_ids) {
        return Err("controller seal bindings do not cover the transition key set");
    }
    if !same_set(&recovery_ids, recovery_key_ids) {
        return Err("recovery seal bindings do not cover the recovery key set");
    }
    let seal_keys = controller_bindings
        .iter()
        .chain(recovery_bindings)
        .map(|binding| binding.seal_xonly)
        .collect::<Vec<_>>();
    if !unique(&seal_keys) {
        return Err("duplicate seal key");
    }
    for key in &seal_keys {
        if forbidden_xonly.iter().any(|forbidden| forbidden == key) {
            return Err("cross-role x-only reuse");
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SealWatch {
    Unspent,
    Spent,
}

pub struct HistoryInput {
    pub has_bitcoin_view: bool,
    pub has_best_block: bool,
    pub observed_height: Option<u32>,
    pub seal: Option<SealWatch>,
    pub spend_proof: bool,
    pub spend_confirmations: u32,
    pub required_depth: u32,
    pub valid_transition: bool,
}

/// Mirrors `identity_history_outcome` in `check_protocol_objects.py`.
pub fn identity_history_state(input: &HistoryInput) -> &'static str {
    if !input.has_bitcoin_view
        || !input.has_best_block
        || input.observed_height.is_none()
        || input.seal.is_none()
    {
        return "INCOMPLETE";
    }
    if input.seal == Some(SealWatch::Unspent) {
        return "CURRENT";
    }
    if input.seal != Some(SealWatch::Spent)
        || !input.spend_proof
        || input.spend_confirmations < input.required_depth
    {
        return "INCOMPLETE";
    }
    if !input.valid_transition {
        return "SEAL_CLOSED_WITHOUT_VALID_TRANSITION";
    }
    "CURRENT"
}

pub fn seal_output_matches(expected: &[u8], actual: &[u8]) -> Result<(), &'static str> {
    if expected == actual {
        Ok(())
    } else {
        Err("seal scriptPubKey mismatch")
    }
}

pub struct RecoveryClock {
    pub not_before_height: u32,
    pub policy_hash: [u8; 32],
    pub prior_policy_hash: [u8; 32],
    pub prior_threshold: u16,
    pub seal_creation_height: u32,
    pub delay_blocks: u32,
    pub block_height: u32,
    pub signer_in_recovery_set: bool,
}

/// Mirrors the object-type 3 branch of `evaluate_signed_payload`.
///
/// A caller-supplied signature count is not an input. One payload cannot
/// satisfy a threshold above one.
pub fn recovery_witness_status(clock: &RecoveryClock) -> Result<&'static str, &'static str> {
    if clock.policy_hash != clock.prior_policy_hash {
        return Err("recovery policy hash does not match the prior state");
    }
    let Some(expected) = clock.seal_creation_height.checked_add(clock.delay_blocks) else {
        return Err("not_before_height does not match seal creation plus delay");
    };
    if clock.not_before_height != expected {
        return Err("not_before_height does not match seal creation plus delay");
    }
    if !clock.signer_in_recovery_set || clock.prior_threshold == 0 {
        return Err("recovery signer is outside the prior policy");
    }
    if clock.prior_threshold > 1 {
        return Err("incomplete");
    }
    if clock.block_height < clock.not_before_height {
        return Err("too_early");
    }
    Ok("valid")
}

pub fn capability_known(capability: u16) -> Result<(), &'static str> {
    if (1..=12).contains(&capability) {
        Ok(())
    } else {
        Err("unknown capability")
    }
}

pub fn state_authorizes(capability: u16, held: &[u16]) -> Result<(), &'static str> {
    if held.contains(&capability) {
        Ok(())
    } else {
        Err("missing state capability")
    }
}

pub fn evidence_ids_valid(ids: &[[u8; 32]]) -> Result<(), &'static str> {
    for pair in ids.windows(2) {
        if pair[0] == pair[1] {
            return Err("duplicate evidence");
        }
        if pair[0] > pair[1] {
            return Err("unsorted evidence");
        }
    }
    Ok(())
}

pub fn increasing_expiry(issued_or_observed: u64, expires: u64) -> Result<(), &'static str> {
    if expires > issued_or_observed {
        Ok(())
    } else {
        Err("expiry does not increase")
    }
}

pub fn observation_time_status(
    observed_at: u64,
    expires_at: u64,
    evaluation_time: Option<u64>,
) -> Result<&'static str, &'static str> {
    increasing_expiry(observed_at, expires_at)?;
    let Some(time) = evaluation_time else {
        return Err("incomplete");
    };
    if time < observed_at {
        return Err("not_yet_observed");
    }
    if time > expires_at {
        return Err("expired");
    }
    Ok("valid")
}

pub fn adapter_scheme(adapter: u8, scheme: u8) -> Result<(), &'static str> {
    if matches!((adapter, scheme), (1, 1) | (2, 2)) {
        Ok(())
    } else {
        Err("unknown adapter key scheme")
    }
}

pub fn adapter_key_distinct(
    adapter_key: &[u8; 32],
    issuer_root: &[u8; 32],
) -> Result<(), &'static str> {
    if adapter_key == issuer_root {
        Err("adapter key reuses the issuer root")
    } else {
        Ok(())
    }
}

pub fn genesis_root_ok(root: &[u8; 32], signer: &[u8; 32]) -> Result<(), &'static str> {
    if root == signer {
        Ok(())
    } else {
        Err("genesis root does not match the signing key")
    }
}

pub fn revocation_target(
    target: &[u8; 32],
    known_entities: &[[u8; 32]],
) -> Result<(), &'static str> {
    if known_entities.contains(target) {
        Err("evidence revocation target is an entity id")
    } else {
        Ok(())
    }
}

pub fn manifest_binding(
    kind: u8,
    signer: &[u8; 32],
    expected_entity: &[u8; 32],
    track_present: bool,
) -> Result<(), &'static str> {
    if signer != expected_entity {
        return Err("manifest signer entity mismatch");
    }
    if kind == 2 && !track_present {
        return Err("album manifest omits its track manifest");
    }
    Ok(())
}

/// Mirrors `package_object_result` in `check_protocol_objects.py`.
pub fn package_object_gap(included: bool) -> &'static str {
    if included {
        "complete"
    } else {
        "incomplete"
    }
}

pub fn header_role_allowed(role: u8) -> Result<(), &'static str> {
    if role == 4 {
        return Err("seal role rejected in signed headers");
    }
    if role > 4 {
        return Err("unknown key role");
    }
    Ok(())
}

fn strictly_sorted(values: &[[u8; 64]]) -> bool {
    !values.is_empty() && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn unique(values: &[[u8; 32]]) -> bool {
    let mut copy = values.to_vec();
    copy.sort();
    copy.windows(2).all(|pair| pair[0] != pair[1])
}

fn same_set(left: &[[u8; 32]], right: &[[u8; 32]]) -> bool {
    let mut a = left.to_vec();
    let mut b = right.to_vec();
    a.sort();
    b.sort();
    a == b
}
