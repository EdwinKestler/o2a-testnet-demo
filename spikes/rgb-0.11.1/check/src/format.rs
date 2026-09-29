//! Verify document and the public package.
//!
//! `package.json` is a hint file this module writes. It is never an input.
//! Entity id, state id, seal outpoint, and official name are recomputed from
//! the signed objects.

use std::fs;
use std::path::Path;

use o2a_demo_core::{
    decode_identity_state, encode_resulting_state, entity_id, evaluate_name_claim, key_id,
    official_name_of, state_id, verify, ClaimAuthorization, SignedObject,
};
use rgbstd::bitcoin::hashes::Hash;
use rgbstd::bitcoin::{OutPoint, Txid};
use serde_json::{json, Value};

pub(crate) const SEAL_SOURCE: &str = "bitcoin-rpc getrawtransaction+getblockheader";

const VERIFY_SCHEMA: &str = include_str!("../../../../docs/formats/verify-v1.schema.json");
const PACKAGE_SCHEMA: &str = include_str!("../../../../docs/formats/public-package-v1.schema.json");
const PACKAGE_FORMAT: &str = "o2a.public-package/v1";
const DENY_MESSAGE: &str = "refusing a field that matches the secret denylist";

const SCHEMA_KEYWORDS: &[&str] = &[
    "$schema",
    "$id",
    "title",
    "description",
    "type",
    "additionalProperties",
    "required",
    "properties",
    "enum",
    "const",
    "pattern",
    "minimum",
    "minLength",
    "items",
];

pub(crate) struct VerifyFacts {
    pub network: String,
    pub verifier_id: String,
    pub entity_id: String,
    pub state_id: String,
    pub identity_history_state: String,
    pub seal_outpoint: String,
    pub confirmations: u64,
    pub required_depth: u32,
    pub unspent: bool,
    pub source: String,
    pub best_block_hash: String,
    pub height: u32,
    pub rgb_status: String,
    pub genesis_valid: bool,
    pub claim_valid: bool,
    pub official_name: Option<String>,
    pub reasons: Vec<String>,
}

pub(crate) fn verifier_id() -> Result<String, String> {
    let id = std::env::var("O2A_VERIFIER_ID").unwrap_or_else(|_| "local".to_string());
    let id = id.trim().to_string();
    if id.is_empty() {
        return Err("verifier_id is empty".into());
    }
    deny_text(&id)?;
    Ok(id)
}

pub(crate) fn seal_text(next_seal: &[u8; 36]) -> String {
    let mut raw = [0u8; 32];
    raw.copy_from_slice(&next_seal[..32]);
    let vout = u32::from_le_bytes([next_seal[32], next_seal[33], next_seal[34], next_seal[35]]);
    format!("{}:{vout}", Txid::from_byte_array(raw))
}

pub(crate) fn outpoint_from_next_seal(next_seal: &[u8; 36]) -> OutPoint {
    let mut raw = [0u8; 32];
    raw.copy_from_slice(&next_seal[..32]);
    let vout = u32::from_le_bytes([next_seal[32], next_seal[33], next_seal[34], next_seal[35]]);
    OutPoint {
        txid: Txid::from_byte_array(raw),
        vout,
    }
}

/// One JSON document. The caller supplies recomputed facts. This function does
/// not read `package.json`, `public.txt`, or the process environment.
pub(crate) fn verification_document(facts: &VerifyFacts) -> Result<String, String> {
    deny_facts(facts)?;
    let value = json!({
        "network": facts.network,
        "verifier_id": facts.verifier_id,
        "entity_id": facts.entity_id,
        "state_id": facts.state_id,
        "identity_history_state": facts.identity_history_state,
        "layers": {
            "bitcoin": {
                "seal_outpoint": facts.seal_outpoint,
                "confirmations": facts.confirmations,
                "required_depth": facts.required_depth,
                "unspent": facts.unspent,
                "source": facts.source,
                "best_block_hash": facts.best_block_hash,
                "height": facts.height
            },
            "rgb": { "status": facts.rgb_status },
            "o2a": {
                "genesis_valid": facts.genesis_valid,
                "claim_valid": facts.claim_valid,
                "official_name": facts.official_name
            }
        },
        "reasons": facts.reasons
    });
    let schema = load_schema(VERIFY_SCHEMA)?;
    check_schema(&schema, &value, "$")?;
    pretty(&value)
}

/// Copy the allow-list into `dest` and write hint `package.json`.
///
/// Source `package.json` is not opened. Extra names are not copied.
pub(crate) fn publish_package(
    source: &Path,
    dest: &Path,
    network_label: &str,
    network_byte: u8,
) -> Result<(), String> {
    deny_text(network_label)?;
    if !matches!(network_label, "regtest" | "signet" | "mainnet") {
        return Err(format!("unknown network {network_label}"));
    }
    prepare_dest(dest)?;
    let genesis_bytes = read_regular(&source.join("genesis.o2a"), "genesis.o2a")?;
    let claim_bytes = read_regular(&source.join("claim.o2a"), "claim.o2a")?;
    deny_bytes(&genesis_bytes)?;
    deny_bytes(&claim_bytes)?;
    let genesis = parse_signed("genesis.o2a", &genesis_bytes)?;
    let claim = parse_signed("claim.o2a", &claim_bytes)?;
    verify(&genesis).map_err(|err| err.to_string())?;
    if genesis.payload.get(2) != Some(&network_byte) {
        return Err("genesis network byte does not match the active profile".into());
    }
    let entity = entity_id(&genesis.payload);
    let state = decode_identity_state(&genesis.payload).map_err(|err| err.to_string())?;
    let sid = state_id(&entity, &encode_resulting_state(&state));
    let controller = state
        .controllers
        .first()
        .ok_or("genesis has no controller")?;
    let kid = key_id(1, controller.xonly);
    let authorization = ClaimAuthorization {
        entity: &entity,
        state: Some(&sid),
        key_id: &kid,
        public_key: &controller.xonly,
        key_role: 1,
        capabilities: &controller.capabilities,
    };
    evaluate_name_claim(
        &claim.payload,
        claim.signature,
        claim.signer_xonly,
        network_byte,
        &authorization,
    )
    .map_err(|err| err.to_string())?;
    let official_name = official_name_of(&claim.payload).map_err(|err| err.to_string())?;
    deny_text(&official_name)?;
    let seal_outpoint = seal_text(&state.next_seal);
    deny_text(&seal_outpoint)?;
    let created_at_height = read_created_at_height(source)?;
    let package = json!({
        "format": PACKAGE_FORMAT,
        "network": network_label,
        "entity_id": hex::encode(entity),
        "state_id": hex::encode(sid),
        "seal_outpoint": seal_outpoint,
        "official_name": official_name,
        "created_at_height": created_at_height
    });
    let schema = load_schema(PACKAGE_SCHEMA)?;
    check_schema(&schema, &package, "$")?;
    let package_text = pretty(&package)?;
    deny_text(&package_text)?;
    fs::write(dest.join("genesis.o2a"), &genesis_bytes).map_err(|err| err.to_string())?;
    fs::write(dest.join("claim.o2a"), &claim_bytes).map_err(|err| err.to_string())?;
    fs::write(dest.join("package.json"), package_text).map_err(|err| err.to_string())?;
    let names = directory_names(dest)?;
    let expected = vec![
        "claim.o2a".to_string(),
        "genesis.o2a".to_string(),
        "package.json".to_string(),
    ];
    if names != expected {
        let _ = fs::remove_dir_all(dest);
        return Err(format!("public package contains {names:?}"));
    }
    for name in &expected {
        deny_bytes(&fs::read(dest.join(name)).map_err(|err| err.to_string())?)?;
    }
    Ok(())
}

pub(crate) fn check_schema(schema: &Value, instance: &Value, path: &str) -> Result<(), String> {
    let schema = schema
        .as_object()
        .ok_or_else(|| format!("schema at {path} is not an object"))?;
    for key in schema.keys() {
        if !SCHEMA_KEYWORDS.contains(&key.as_str()) {
            return Err(format!("schema keyword is not checked: {key}"));
        }
    }
    if let Some(spec) = schema.get("type") {
        if !type_matches(spec, instance) {
            return Err(format!("type mismatch at {path}"));
        }
    }
    if let Some(expected) = schema.get("const") {
        if instance != expected {
            return Err(format!("const mismatch at {path}"));
        }
    }
    if let Some(enum_values) = schema.get("enum").and_then(Value::as_array) {
        if !enum_values.iter().any(|item| item == instance) {
            return Err(format!("enum mismatch at {path}"));
        }
    }
    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
        let text = instance
            .as_str()
            .ok_or_else(|| format!("pattern target at {path} is not a string"))?;
        if !pattern_matches(pattern, text)? {
            return Err(format!("pattern mismatch at {path}"));
        }
    }
    if let Some(min_length) = schema.get("minLength").and_then(Value::as_u64) {
        let text = instance
            .as_str()
            .ok_or_else(|| format!("minLength target at {path} is not a string"))?;
        if (text.chars().count() as u64) < min_length {
            return Err(format!("minLength mismatch at {path}"));
        }
    }
    if let Some(minimum) = schema.get("minimum") {
        if !minimum_holds(minimum, instance) {
            return Err(format!("minimum mismatch at {path}"));
        }
    }
    if schema.contains_key("properties") || schema.contains_key("additionalProperties") {
        match schema.get("additionalProperties") {
            Some(Value::Bool(false)) | None => {}
            _ => return Err(format!("additionalProperties must be false at {path}")),
        }
        let instance_object = instance
            .as_object()
            .ok_or_else(|| format!("type mismatch at {path}"))?;
        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("schema properties missing at {path}"))?;
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for item in required {
                let name = item
                    .as_str()
                    .ok_or_else(|| format!("schema required entry at {path} is not a string"))?;
                if !instance_object.contains_key(name) {
                    return Err(format!("missing field {path}.{name}"));
                }
            }
        }
        for key in instance_object.keys() {
            if !properties.contains_key(key) {
                return Err(format!("unexpected field {path}.{key}"));
            }
        }
        for (key, child) in properties {
            if let Some(value) = instance_object.get(key) {
                check_schema(child, value, &format!("{path}.{key}"))?;
            }
        }
    }
    if let Some(items) = schema.get("items") {
        let values = instance
            .as_array()
            .ok_or_else(|| format!("type mismatch at {path}"))?;
        for (index, value) in values.iter().enumerate() {
            check_schema(items, value, &format!("{path}[{index}]"))?;
        }
    }
    Ok(())
}

fn deny_facts(facts: &VerifyFacts) -> Result<(), String> {
    deny_text(&facts.network)?;
    deny_text(&facts.verifier_id)?;
    deny_text(&facts.entity_id)?;
    deny_text(&facts.state_id)?;
    deny_text(&facts.identity_history_state)?;
    deny_text(&facts.seal_outpoint)?;
    deny_text(&facts.source)?;
    deny_text(&facts.best_block_hash)?;
    deny_text(&facts.rgb_status)?;
    if let Some(name) = &facts.official_name {
        deny_text(name)?;
    }
    for reason in &facts.reasons {
        deny_text(reason)?;
    }
    Ok(())
}

fn deny_text(value: &str) -> Result<(), String> {
    if secret_marked(value) {
        Err(DENY_MESSAGE.into())
    } else {
        Ok(())
    }
}

fn deny_bytes(bytes: &[u8]) -> Result<(), String> {
    deny_text(&String::from_utf8_lossy(bytes))
}

fn secret_marked(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if ["xprv", "tprv", "mnemonic", "seed.hex", "o2a_demo_seed"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return true;
    }
    let normalized = lower.replace('\\', "/");
    if !normalized.contains('/') {
        return false;
    }
    normalized.split('/').any(|segment| {
        let segment =
            segment.trim_matches(|ch: char| matches!(ch, '"' | '\'' | ' ' | '\t' | ',' | ':'));
        segment == "seed" || segment.starts_with("seed.")
    })
}

fn load_schema(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|err| err.to_string())
}

fn pretty(value: &Value) -> Result<String, String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

fn prepare_dest(dest: &Path) -> Result<(), String> {
    match fs::symlink_metadata(dest) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err("destination is a symlink".into());
            }
            if !meta.is_dir() {
                return Err("destination is not a directory".into());
            }
            let mut entries = fs::read_dir(dest).map_err(|err| err.to_string())?;
            if entries.next().is_some() {
                return Err("destination is not empty".into());
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(dest).map_err(|err| err.to_string())?;
        }
        Err(err) => return Err(err.to_string()),
    }
    Ok(())
}

fn read_regular(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let meta = fs::symlink_metadata(path).map_err(|_| format!("{name} is missing"))?;
    if meta.file_type().is_symlink() {
        return Err(format!("{name} is a symlink"));
    }
    if !meta.is_file() {
        return Err(format!("{name} is not a regular file"));
    }
    fs::read(path).map_err(|err| format!("{name}: {err}"))
}

fn parse_signed(name: &str, bytes: &[u8]) -> Result<SignedObject, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| format!("{name} is not utf-8"))?;
    crate::lineage::parse_object(text)
}

fn read_created_at_height(source: &Path) -> Result<u32, String> {
    let path = source.join("public.txt");
    let meta = fs::symlink_metadata(&path).map_err(|_| "public.txt is missing".to_string())?;
    if meta.file_type().is_symlink() {
        return Err("public.txt is a symlink".into());
    }
    if !meta.is_file() {
        return Err("public.txt is not a regular file".into());
    }
    let bytes = fs::read(&path).map_err(|err| err.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    let mut found = None;
    for line in text.lines() {
        let Some(value) = line.strip_prefix("created_at_height=") else {
            continue;
        };
        if found.is_some() {
            return Err("created_at_height is repeated".into());
        }
        let height = value
            .parse::<u32>()
            .map_err(|_| "created_at_height is not a u32")?;
        found = Some(height);
    }
    found.ok_or_else(|| "public.txt has no created_at_height".into())
}

fn directory_names(dir: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

fn type_matches(spec: &Value, instance: &Value) -> bool {
    match spec {
        Value::String(name) => json_type(name, instance),
        Value::Array(options) => options.iter().any(|item| type_matches(item, instance)),
        _ => false,
    }
}

fn json_type(name: &str, instance: &Value) -> bool {
    match name {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        "integer" => is_json_integer(instance),
        "number" => instance.is_number(),
        _ => false,
    }
}

fn is_json_integer(instance: &Value) -> bool {
    instance.as_i64().is_some() || instance.as_u64().is_some()
}

fn pattern_matches(pattern: &str, value: &str) -> Result<bool, String> {
    let ok = match pattern {
        "^[0-9a-f]{64}$" => is_lower_hex(value, 64),
        "^[0-9a-f]{64}:[0-9]+$" => match value.split_once(':') {
            Some((left, right)) => {
                is_lower_hex(left, 64)
                    && !right.is_empty()
                    && right.bytes().all(|byte| byte.is_ascii_digit())
            }
            None => false,
        },
        other => return Err(format!("schema pattern is not checked: {other}")),
    };
    Ok(ok)
}

fn is_lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn minimum_holds(minimum: &Value, instance: &Value) -> bool {
    let Some(minimum) = minimum.as_i64() else {
        return false;
    };
    if let Some(value) = instance.as_i64() {
        return value >= minimum;
    }
    if let Some(value) = instance.as_u64() {
        return i64::try_from(value)
            .map(|value| value >= minimum)
            .unwrap_or(true);
    }
    false
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use std::time::{SystemTime, UNIX_EPOCH};

    use o2a_demo_core::{
        encode_resulting_state, entity_id, genesis_for, genesis_state_from, keys_for,
        official_name_claim, official_name_nonce, state_id,
    };
    use rgbstd::bitcoin::hashes::Hash;
    use rgbstd::bitcoin::Txid;
    use serde_json::Value;

    use super::*;

    const HISTORY_STATES: [&str; 5] = [
        "CURRENT",
        "PENDING_CONFIRMATION",
        "INCOMPLETE",
        "INVALID",
        "SEAL_CLOSED_WITHOUT_VALID_TRANSITION",
    ];
    const MARKER: &str = "SECRET-SEED-MARKER";

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "o2a-fmt-{}-{}-{}",
                std::process::id(),
                name,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&path).expect("scratch");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample(state: &str) -> VerifyFacts {
        VerifyFacts {
            network: "signet".into(),
            verifier_id: "local".into(),
            entity_id: "ab".repeat(32),
            state_id: "cd".repeat(32),
            identity_history_state: state.into(),
            seal_outpoint: format!("{}:0", "11".repeat(32)),
            confirmations: 6,
            required_depth: 6,
            unspent: state == "CURRENT",
            source: SEAL_SOURCE.into(),
            best_block_hash: "ef".repeat(32),
            height: 106,
            rgb_status: "Consignment is valid".into(),
            genesis_valid: true,
            claim_valid: state == "CURRENT",
            official_name: if state == "CURRENT" {
                Some("Rehearsal Name".into())
            } else {
                None
            },
            reasons: if state == "CURRENT" {
                Vec::new()
            } else {
                vec![format!("identity_history_state={state}")]
            },
        }
    }

    fn required_names(schema_text: &str) -> Vec<String> {
        let schema: Value = serde_json::from_str(schema_text).expect("schema");
        schema["required"]
            .as_array()
            .expect("required")
            .iter()
            .map(|item| item.as_str().expect("name").to_string())
            .collect()
    }

    #[test]
    fn verify_schema_lists_the_required_fields() {
        assert_eq!(
            required_names(VERIFY_SCHEMA),
            [
                "network",
                "verifier_id",
                "entity_id",
                "state_id",
                "identity_history_state",
                "layers",
                "reasons"
            ]
        );
        let schema: Value = serde_json::from_str(VERIFY_SCHEMA).expect("schema");
        let states: Vec<&str> = schema["properties"]["identity_history_state"]["enum"]
            .as_array()
            .expect("enum")
            .iter()
            .map(|item| item.as_str().expect("state"))
            .collect();
        assert_eq!(states, HISTORY_STATES);
    }

    #[test]
    fn package_schema_lists_the_hint_fields() {
        assert_eq!(
            required_names(PACKAGE_SCHEMA),
            [
                "format",
                "network",
                "entity_id",
                "state_id",
                "seal_outpoint",
                "official_name",
                "created_at_height"
            ]
        );
    }

    #[test]
    fn verification_document_accepts_each_history_state() {
        let schema = load_schema(VERIFY_SCHEMA).expect("schema");
        for state in HISTORY_STATES {
            let text = verification_document(&sample(state)).expect(state);
            assert!(text.ends_with('\n'), "{state}");
            let value: Value = serde_json::from_str(&text).expect(state);
            check_schema(&schema, &value, "$").expect(state);
            assert_eq!(value["identity_history_state"], state);
            assert_eq!(value["entity_id"], "ab".repeat(32));
            assert_eq!(
                value["layers"]["bitcoin"]["source"],
                "bitcoin-rpc getrawtransaction+getblockheader"
            );
            assert!(value.get("package.json").is_none());
        }
    }

    #[test]
    fn verification_document_keeps_a_missing_name_null() {
        let mut facts = sample("CURRENT");
        facts.claim_valid = false;
        facts.official_name = None;
        facts.reasons = vec!["official_name claim is absent".into()];
        let text = verification_document(&facts).expect("document");
        let value: Value = serde_json::from_str(&text).expect("json");
        assert!(value["layers"]["o2a"]["official_name"].is_null());
        assert_eq!(value["layers"]["o2a"]["claim_valid"], false);
        assert_eq!(value["reasons"][0], "official_name claim is absent");
    }

    #[test]
    fn verification_document_rejects_an_extra_field_and_a_copied_hint() {
        let text = verification_document(&sample("CURRENT")).expect("document");
        let mut value: Value = serde_json::from_str(&text).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("entity_id_hint".into(), json!("11".repeat(32)));
        let schema = load_schema(VERIFY_SCHEMA).expect("schema");
        let err = check_schema(&schema, &value, "$").expect_err("extra");
        assert!(err.contains("unexpected field"), "{err}");
        assert!(err.contains("entity_id_hint"), "{err}");
        assert!(!err.contains(&"11".repeat(32)), "{err}");
    }

    #[test]
    fn verification_document_refuses_secret_markers_without_echoing_them() {
        let secret = "xprv-UNIQUE-MARKER";
        let mut facts = sample("CURRENT");
        facts.verifier_id = secret.into();
        let err = verification_document(&facts).expect_err("denylist");
        assert_eq!(err, DENY_MESSAGE);
        assert!(!err.contains("UNIQUE-MARKER"));
        assert!(!err.contains("xprv"));

        facts.verifier_id = "local".into();
        facts.official_name = Some("Seed".into());
        verification_document(&facts).expect("a bare Seed word is not a path");
        facts.official_name = Some("notes/seed".into());
        assert_eq!(
            verification_document(&facts).expect_err("path"),
            DENY_MESSAGE
        );
        facts.official_name = Some("wallet/seed.hex".into());
        assert_eq!(
            verification_document(&facts).expect_err("seed file"),
            DENY_MESSAGE
        );
    }

    #[test]
    fn seal_text_uses_the_signed_outpoint_bytes() {
        let txid = Txid::from_str(&"11".repeat(32)).expect("txid");
        let mut seal = [0u8; 36];
        seal[..32].copy_from_slice(&txid.to_byte_array());
        seal[32..].copy_from_slice(&7u32.to_le_bytes());
        assert_eq!(seal_text(&seal), format!("{}:7", "11".repeat(32)));
        let point = outpoint_from_next_seal(&seal);
        assert_eq!(point.txid, txid);
        assert_eq!(point.vout, 7);
    }

    fn signed_pair(dir: &Path) -> (String, String, String) {
        let _lock = crate::test_env::lock_env(&[
            ("O2A_DEMO_SEED_FILE", None),
            ("O2A_DEMO_ENTITY", Some("7")),
            ("O2A_DEMO_DELAY", None),
            ("O2A_DEMO_THRESHOLD", None),
            ("O2A_DEMO_NETWORK", None),
            ("O2A_NETWORK", None),
            ("O2A_VERIFIER_ID", None),
        ]);
        let txid = Txid::from_str(&"11".repeat(32)).expect("txid");
        let mut seal = [0u8; 36];
        seal[..32].copy_from_slice(&txid.to_byte_array());
        seal[32..].copy_from_slice(&7u32.to_le_bytes());
        let keys = keys_for(1);
        let state = genesis_state_from(&keys, seal);
        let genesis = genesis_for(3, keys.root, &state);
        let entity = entity_id(&genesis.payload);
        let decoded = decode_identity_state(&genesis.payload).expect("state");
        let sid = state_id(&entity, &encode_resulting_state(&decoded));
        let name = "Rehearsal Name";
        let claim = official_name_claim(
            3,
            entity,
            sid,
            keys.controller_0,
            name,
            official_name_nonce(&entity, name),
        )
        .expect("claim");
        fs::write(dir.join("genesis.o2a"), object_text(&genesis)).expect("genesis");
        fs::write(dir.join("claim.o2a"), object_text(&claim)).expect("claim");
        (
            hex::encode(entity),
            hex::encode(sid),
            seal_text(&decoded.next_seal),
        )
    }

    fn object_text(object: &SignedObject) -> String {
        format!(
            "tag={}\npayload={}\ndigest={}\nsigner={}\nsignature={}\n",
            object.tag,
            hex::encode(&object.payload),
            hex::encode(object.digest),
            hex::encode(object.signer_xonly),
            hex::encode(object.signature)
        )
    }

    fn plant_secrets(dir: &Path) {
        fs::write(dir.join("seed.hex"), format!("{MARKER}\n")).expect("seed");
        fs::write(dir.join("wallet.dat"), format!("wallet {MARKER}")).expect("wallet");
        fs::write(
            dir.join("bitcoin.conf"),
            format!("rpcuser=reader\nrpcpassword={MARKER}\n"),
        )
        .expect("rpc");
        fs::write(dir.join(".cookie"), format!("__cookie__:{MARKER}")).expect("cookie");
        fs::write(
            dir.join("package.json"),
            format!(
                "{{\"entity_id\":\"{}\",\"network\":\"mainnet\",\"note\":\"{MARKER}\"}}\n",
                "aa".repeat(32)
            ),
        )
        .expect("lying package");
        fs::write(
            dir.join("public.txt"),
            format!(
                "entity_id={}\nstate_id={}\nseal={}:1\nconfirmations=1\nnetwork=mainnet\ncreated_at_height=42\n{MARKER}\n",
                "11".repeat(32),
                "22".repeat(32),
                "ff".repeat(32)
            ),
        )
        .expect("public");
        symlink(dir.join("seed.hex"), dir.join("leak")).expect("symlink");
        let zero = fs::Permissions::from_mode(0o000);
        fs::set_permissions(dir.join("package.json"), zero.clone()).expect("package mode");
        fs::set_permissions(dir.join("seed.hex"), zero).expect("seed mode");
    }

    #[test]
    fn publish_package_copies_only_the_allow_list_and_recomputes_hints() {
        let source = Scratch::new("src");
        let dest = Scratch::new("dst");
        let (entity, state, seal) = signed_pair(source.path());
        plant_secrets(source.path());
        publish_package(source.path(), dest.path(), "signet", 3).expect("publish");
        let mut names = directory_names(dest.path()).expect("names");
        names.sort();
        assert_eq!(
            names,
            vec![
                "claim.o2a".to_string(),
                "genesis.o2a".to_string(),
                "package.json".to_string()
            ]
        );
        let package_bytes = fs::read(dest.path().join("package.json")).expect("package");
        let package_text = String::from_utf8(package_bytes).expect("utf8");
        assert!(!package_text.contains(MARKER), "{package_text}");
        assert!(!package_text.contains(&"aa".repeat(32)), "{package_text}");
        assert!(!package_text.contains("mainnet"), "{package_text}");
        assert!(!package_text.contains(&"ff".repeat(32)), "{package_text}");
        let package: Value = serde_json::from_str(&package_text).expect("json");
        let schema = load_schema(PACKAGE_SCHEMA).expect("schema");
        check_schema(&schema, &package, "$").expect("package schema");
        assert_eq!(package["format"], PACKAGE_FORMAT);
        assert_eq!(package["network"], "signet");
        assert_eq!(package["entity_id"], entity);
        assert_eq!(package["state_id"], state);
        assert_eq!(package["seal_outpoint"], seal);
        assert_eq!(package["official_name"], "Rehearsal Name");
        assert_eq!(package["created_at_height"], 42);
        for name in ["genesis.o2a", "claim.o2a", "package.json"] {
            let bytes = fs::read(dest.path().join(name)).expect(name);
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains(MARKER), "{name}");
            assert!(!text.contains("xprv"), "{name}");
            assert!(!text.contains("rpcpassword"), "{name}");
        }
    }

    #[test]
    fn publish_package_refuses_symlinks_secrets_and_a_used_destination() {
        let source = Scratch::new("bad-src");
        let dest = Scratch::new("bad-dst");
        signed_pair(source.path());
        fs::write(source.path().join("seed.hex"), format!("xprv{MARKER}")).expect("seed");
        fs::rename(
            source.path().join("genesis.o2a"),
            source.path().join("genesis-real.o2a"),
        )
        .expect("rename");
        symlink(
            source.path().join("genesis-real.o2a"),
            source.path().join("genesis.o2a"),
        )
        .expect("genesis symlink");
        let err = publish_package(source.path(), dest.path(), "signet", 3).expect_err("symlink");
        assert_eq!(err, "genesis.o2a is a symlink");
        assert!(!dest_contains_marker(dest.path()));

        fs::remove_file(source.path().join("genesis.o2a")).expect("unlink");
        fs::rename(
            source.path().join("genesis-real.o2a"),
            source.path().join("genesis.o2a"),
        )
        .expect("restore");
        let mut genesis = fs::read_to_string(source.path().join("genesis.o2a")).expect("read");
        genesis.push_str("\nxprv-appended\n");
        fs::write(source.path().join("genesis.o2a"), genesis).expect("tamper");
        fs::write(source.path().join("public.txt"), "created_at_height=1\n").expect("height");
        let err = publish_package(source.path(), dest.path(), "signet", 3).expect_err("secret");
        assert_eq!(err, DENY_MESSAGE);
        assert!(!err.contains(MARKER));
        assert!(!err.contains("xprv"));
        assert!(!dest_contains_marker(dest.path()));

        let clean = Scratch::new("clean-src");
        let used = Scratch::new("used-dst");
        signed_pair(clean.path());
        fs::write(clean.path().join("public.txt"), "created_at_height=9\n").expect("height");
        fs::write(used.path().join("note.txt"), MARKER).expect("note");
        let err = publish_package(clean.path(), used.path(), "signet", 3).expect_err("used");
        assert_eq!(err, "destination is not empty");
        assert_eq!(
            fs::read_to_string(used.path().join("note.txt")).expect("note"),
            MARKER
        );
        assert!(directory_names(used.path()).expect("names") == vec!["note.txt".to_string()]);

        fs::remove_file(clean.path().join("claim.o2a")).expect("claim");
        let empty = Scratch::new("empty-dst");
        let err = publish_package(clean.path(), empty.path(), "signet", 3).expect_err("claim");
        assert_eq!(err, "claim.o2a is missing");

        let no_height = Scratch::new("no-height");
        signed_pair(no_height.path());
        fs::write(
            no_height.path().join("public.txt"),
            format!("entity_id={}\nnetwork=mainnet\n", "11".repeat(32)),
        )
        .expect("public");
        let err = publish_package(no_height.path(), empty.path(), "signet", 3).expect_err("height");
        assert_eq!(err, "public.txt has no created_at_height");
        assert!(directory_names(empty.path()).expect("empty").is_empty());
    }

    fn dest_contains_marker(dir: &Path) -> bool {
        let Ok(names) = directory_names(dir) else {
            return false;
        };
        names.iter().any(|name| {
            fs::read(dir.join(name))
                .map(|bytes| String::from_utf8_lossy(&bytes).contains(MARKER))
                .unwrap_or(false)
        })
    }
}
