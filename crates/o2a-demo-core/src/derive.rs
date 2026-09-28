//! BIP85 and hardened identity derivation for the disposable demo.
//!
//! Paths are below `xprv_o2a`, never the wallet master. Role `4'` is the seal
//! role. Payment paths are not O2A identity keys.

use secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};

use crate::DemoKey;

const HARDENED: u32 = 1 << 31;
const O2A_INDEX: u32 = 998_536_622;

#[derive(Clone, Copy)]
struct Xprv {
    secret: [u8; 32],
    chain_code: [u8; 32],
}

pub fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    use bitcoin_hashes::{
        hmac::{Hmac, HmacEngine},
        sha512, Hash, HashEngine,
    };
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
        data[..33]
            .copy_from_slice(&PublicKey::from_secret_key(&Secp256k1::new(), &key).serialize());
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

pub fn hard(index: u32) -> u32 {
    assert!(index < HARDENED, "demo index must fit in 31 bits");
    index | HARDENED
}

fn derive(mut key: Xprv, path: &[u32]) -> Xprv {
    for index in path {
        key = child(key, *index);
    }
    key
}

fn bip85_o2a(master_key: Xprv) -> Xprv {
    let key = derive(master_key, &[hard(83_696_968), hard(32), hard(O2A_INDEX)]);
    let digest = hmac_sha512(b"bip-entropy-from-k", &key.secret);
    Xprv {
        chain_code: digest[..32].try_into().expect("fixed hash length"),
        secret: digest[32..].try_into().expect("fixed hash length"),
    }
}

fn demo_key(xprv: Xprv) -> DemoKey {
    let secret = SecretKey::from_slice(&xprv.secret).expect("valid derived key");
    let pair = secp256k1::Keypair::from_secret_key(&Secp256k1::new(), &secret);
    let (xonly, _) = pair.x_only_public_key();
    DemoKey {
        secret: xprv.secret,
        xonly: xonly.serialize(),
    }
}

fn o2a_root(seed: &[u8]) -> Xprv {
    bip85_o2a(master(seed))
}

/// Derives `m/coin'/entity'/role'/index'` below `xprv_o2a`.
pub fn identity_key(seed: &[u8], coin: u32, entity: u32, role: u32, index: u32) -> DemoKey {
    demo_key(identity_xprv(seed, coin, entity, role, index))
}

#[cfg(test)]
fn xprv_bytes(key: Xprv) -> [u8; 64] {
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&key.secret);
    out[32..].copy_from_slice(&key.chain_code);
    out
}

#[cfg(test)]
pub(crate) fn o2a_xprv_bytes(seed: &[u8]) -> [u8; 64] {
    xprv_bytes(o2a_root(seed))
}

#[cfg(test)]
pub(crate) fn identity_xprv_bytes(
    seed: &[u8],
    coin: u32,
    entity: u32,
    role: u32,
    index: u32,
) -> [u8; 64] {
    xprv_bytes(identity_xprv(seed, coin, entity, role, index))
}

fn identity_xprv(seed: &[u8], coin: u32, entity: u32, role: u32, index: u32) -> Xprv {
    derive(
        o2a_root(seed),
        &[hard(coin), hard(entity), hard(role), hard(index)],
    )
}

/// Accepts a v0.1 identity path and candidate x-only key, or returns the
/// vector reason. `network` is `mainnet` or `regtest`.
pub fn accept_identity_key(
    profile_version: u16,
    network: &str,
    path: &str,
    candidate: &[u8; 32],
    seed: &[u8],
) -> Result<(), &'static str> {
    if profile_version != 1 {
        return Err("unknown derivation profile version");
    }
    let parts = parse_path(path)?;
    if parts.first().is_some_and(|step| step.0 == 827 && step.1) {
        return Err("retired purpose path is outside Route B");
    }
    if parts.first().is_some_and(|step| step.0 == 86 && step.1) {
        return Err("payment keys have no O2A signing role");
    }
    if parts.len() != 4 {
        return Err("identity path must contain coin, entity, role, and index");
    }
    let coin = hardened(parts[0], "coin type")?;
    let entity = hardened(parts[1], "entity")?;
    let role = match parts[2] {
        (_, false) => return Err("identity role must be hardened"),
        (value, true) => value,
    };
    let index = match parts[3] {
        (_, false) => return Err("identity key index must be hardened"),
        (value, true) => value,
    };
    let expected_coin: u32 = match network {
        "mainnet" => 0,
        "testnet" | "testnet4" | "signet" | "regtest" => 1,
        _ => return Err("unknown network"),
    };
    if coin != expected_coin {
        return Err("coin type does not match declared network");
    }
    if role == 0 && index != 0 {
        return Err("root terminal index is fixed at zero");
    }
    if role > 4 {
        return Err("unknown identity role");
    }
    let derived = identity_key(seed, coin, entity, role, index);
    if derived.xonly == *candidate {
        return Ok(());
    }
    let entity_zero_root = identity_key(seed, coin, 0, 0, 0).xonly;
    if entity != 0 && *candidate == entity_zero_root {
        return Err("entity 0 key cannot fill entity 1 path");
    }
    let this_root = identity_key(seed, coin, entity, 0, 0).xonly;
    if role != 0 && *candidate == this_root {
        return Err("root key cannot fill controller role");
    }
    Err("candidate x-only key does not match the identity path")
}

fn hardened(step: (u32, bool), name: &'static str) -> Result<u32, &'static str> {
    if step.1 {
        Ok(step.0)
    } else {
        Err(match name {
            "coin type" => "coin type must be hardened",
            "entity" => "entity index must be hardened",
            _ => "identity component must be hardened",
        })
    }
}

/// Returns `(index, hardened)` for each path component after `m`.
fn parse_path(path: &str) -> Result<Vec<(u32, bool)>, &'static str> {
    let rest = path
        .strip_prefix("m/")
        .ok_or("identity path must start at m")?;
    if rest.is_empty() {
        return Err("identity path is empty");
    }
    rest.split('/')
        .map(|step| {
            let (body, hardened) = if let Some(body) = step.strip_suffix('\'') {
                (body, true)
            } else if let Some(body) = step.strip_suffix('h') {
                (body, true)
            } else {
                (step, false)
            };
            let index = body.parse::<u32>().map_err(|_| "invalid path index")?;
            if index >= HARDENED {
                return Err("path index exceeds 31 bits");
            }
            Ok((index, hardened))
        })
        .collect()
}
