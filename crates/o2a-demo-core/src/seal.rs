//! Deterministic P2TR seal script from a seal policy.
//!
//! The internal key is BIP341 NUMS point H. There is no key-path spend.
//! Leaves are version `0xc0`, controller keys then the recovery leaf, reduced
//! pairwise from the left.

use secp256k1::{Parity, PublicKey, Scalar, Secp256k1, XOnlyPublicKey};

use crate::encode::SealBinding;
use crate::tagged_hash;

pub const NUMS_X: [u8; 32] = [
    0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54, 0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9, 0x7a, 0x5e,
    0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5, 0x47, 0xbf, 0xee, 0x9a, 0xce, 0x80, 0x3a, 0xc0,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealScript {
    pub policy: Vec<u8>,
    pub scripts: Vec<Vec<u8>>,
    pub leaf_hashes: Vec<[u8; 32]>,
    pub merkle_root: [u8; 32],
    pub output_key: [u8; 32],
    pub script_pubkey: Vec<u8>,
}

pub fn script_num(value: u64) -> Result<Vec<u8>, &'static str> {
    if value == 0 {
        return Ok(vec![0x00]);
    }
    if (1..=16).contains(&value) {
        return Ok(vec![0x50 + value as u8]);
    }
    let mut data = Vec::new();
    let mut number = value;
    while number > 0 {
        data.push((number & 0xff) as u8);
        number >>= 8;
    }
    if data.last().is_some_and(|byte| byte & 0x80 != 0) {
        data.push(0);
    }
    if data.len() > 75 {
        return Err("script number is too wide");
    }
    let mut out = Vec::with_capacity(1 + data.len());
    out.push(data.len() as u8);
    out.extend(data);
    Ok(out)
}

pub fn recovery_leaf(
    keys: &[[u8; 32]],
    threshold: u64,
    delay: u64,
) -> Result<Vec<u8>, &'static str> {
    let mut script = Vec::new();
    for (index, key) in keys.iter().enumerate() {
        script.push(0x20);
        script.extend_from_slice(key);
        script.push(if index == 0 { 0xac } else { 0xba });
    }
    script.extend(script_num(threshold)?);
    script.push(0x9d);
    script.extend(script_num(delay)?);
    script.push(0xb2);
    Ok(script)
}

fn compact_size(value: usize) -> Vec<u8> {
    if value < 253 {
        vec![value as u8]
    } else {
        let mut out = vec![0xfd];
        out.extend_from_slice(&(value as u16).to_le_bytes());
        out
    }
}

pub fn tapleaf_hash(script: &[u8]) -> [u8; 32] {
    let mut preimage = vec![0xc0];
    preimage.extend(compact_size(script.len()));
    preimage.extend_from_slice(script);
    tagged_hash("TapLeaf", &preimage)
}

pub fn taproot_root(leaves: &[[u8; 32]]) -> Result<[u8; 32], &'static str> {
    if leaves.is_empty() {
        return Err("empty seal tree");
    }
    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        let mut index = 0;
        while index < current.len() {
            if index + 1 == current.len() {
                next.push(current[index]);
                index += 1;
                continue;
            }
            let (left, right) = if current[index] <= current[index + 1] {
                (current[index], current[index + 1])
            } else {
                (current[index + 1], current[index])
            };
            let mut preimage = [0u8; 64];
            preimage[..32].copy_from_slice(&left);
            preimage[32..].copy_from_slice(&right);
            next.push(tagged_hash("TapBranch", &preimage));
            index += 2;
        }
        current = next;
    }
    Ok(current[0])
}

pub fn seal_script(
    controller_bindings: &[SealBinding],
    recovery_bindings: &[SealBinding],
    threshold: u64,
    delay: u64,
    policy: Vec<u8>,
) -> Result<SealScript, &'static str> {
    let mut controller_keys = controller_bindings
        .iter()
        .map(|binding| binding.seal_xonly)
        .collect::<Vec<_>>();
    controller_keys.sort();
    let mut recovery_keys = recovery_bindings
        .iter()
        .map(|binding| binding.seal_xonly)
        .collect::<Vec<_>>();
    recovery_keys.sort();
    let mut scripts = controller_keys
        .iter()
        .map(|key| {
            let mut script = vec![0x20];
            script.extend_from_slice(key);
            script.push(0xac);
            script
        })
        .collect::<Vec<_>>();
    scripts.push(recovery_leaf(&recovery_keys, threshold, delay)?);
    let leaf_hashes = scripts
        .iter()
        .map(|script| tapleaf_hash(script))
        .collect::<Vec<_>>();
    let merkle_root = taproot_root(&leaf_hashes)?;
    let mut tweak_preimage = NUMS_X.to_vec();
    tweak_preimage.extend_from_slice(&merkle_root);
    let tweak = tagged_hash("TapTweak", &tweak_preimage);
    let scalar = Scalar::from_be_bytes(tweak).map_err(|_| "TapTweak exceeds group order")?;
    let secp = Secp256k1::new();
    let internal = XOnlyPublicKey::from_slice(&NUMS_X).expect("NUMS x coordinate");
    let point = PublicKey::from_x_only_public_key(internal, Parity::Even);
    let tweaked = point
        .add_exp_tweak(&secp, &scalar)
        .map_err(|_| "taproot tweak produced the point at infinity")?;
    let (output_key, _) = tweaked.x_only_public_key();
    let output_key = output_key.serialize();
    let mut script_pubkey = vec![0x51, 0x20];
    script_pubkey.extend_from_slice(&output_key);
    Ok(SealScript {
        policy,
        scripts,
        leaf_hashes,
        merkle_root,
        output_key,
        script_pubkey,
    })
}
