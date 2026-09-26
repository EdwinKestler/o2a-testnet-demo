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
