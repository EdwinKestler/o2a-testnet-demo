//! Byte-for-byte checks against the pinned sibling specification commit.
//!
//! The vectors are read with `git show <SPEC_COMMIT>:path`. A missing sibling
//! repository or commit fails the test.

use std::path::PathBuf;
use std::process::Command;

use crate::decode::{decode_payload, evaluate_name_claim, parse_claim, ClaimAuthorization};
use crate::encode::{
    bytes_field, common_header, content_reference, encode_recovery_policy, encode_resulting_state,
    encode_seal_policy, key_id, list_items, option_fixed, text_field, ControllerEntry,
    RecoveryPolicy, ResultingState, SealBinding,
};
use crate::seal::seal_script;
use crate::{
    accept_identity_key, entity_id, entity_id_checked, identity_key, recovery_policy_hash,
    signature_accepts, tagged_hash, SPEC_COMMIT, UNSAFE_BIP39_SEED_HEX,
};
use crate::{
    adapter_key_distinct, adapter_scheme, capability_known, evidence_ids_valid, genesis_root_ok,
    identity_history_state, increasing_expiry, manifest_binding, observation_time_status,
    package_object_gap, recovery_policy_valid, recovery_witness_status, revocation_target,
    seal_output_matches, seal_policy_valid, state_authorizes, HistoryInput, RecoveryClock,
    SealWatch,
};

fn protocol_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../o2a-protocol")
}

fn git_show(path: &str) -> String {
    let repo = protocol_repo();
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["show", &format!("{SPEC_COMMIT}:{path}")])
        .output()
        .unwrap_or_else(|err| panic!("git show failed for {path}: {err}"));
    assert!(
        output.status.success(),
        "spec vector {path} is missing at {SPEC_COMMIT} in {}: {}",
        repo.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("vector is utf-8")
}

fn hx(value: &str) -> Vec<u8> {
    hex::decode(value).unwrap_or_else(|err| panic!("hex {value}: {err}"))
}

fn hx32(value: &str) -> [u8; 32] {
    hx(value).try_into().expect("32 bytes")
}

fn marker_outpoint(marker: u8, index: u32) -> [u8; 36] {
    let mut out = [marker; 36];
    out[32..].copy_from_slice(&index.to_le_bytes());
    out
}

fn caps() -> Vec<u16> {
    vec![2, 4, 5, 6, 7, 8, 9, 10, 11, 12]
}

fn protocol_payloads() -> Vec<(&'static str, Vec<u8>)> {
    let root = hx32("f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9");
    let controller = hx32("dff1d77f2a671c5f36183726db2341be58feae1da2deced843240f7b502ba659");
    let recovery = hx32("dd308afec5777e13121fa72b9cc1b7cc0139715309b086c960e18fd969774eb8");
    let album = hx32("25d1dff95105f5253c4022f628a996ad3a0d95fbf21d468a1b33f8c160d8f517");
    let nostr = hx32("d69c3509bb99e412e68b0fe8544e72837dfa30746d8be2aa65975f29d22dc7b9");
    let seal0 = hx32("d25ed00ba7188d413f7c0ed100bb092460be1eaed2e69596c3dfc43dc43e5c06");
    let seal3 = hx32("6e5382b91922ab39a3995429c0a60fd2caa301c060cb9b2b428f20865e9d11d8");
    let state = [0x22u8; 32];
    let next_state = [0x23u8; 32];
    let root_entity = entity_id(root);
    let event_entity = entity_id(recovery);
    let album_entity = entity_id(album);
    let recovery_id = key_id(2, recovery);
    let policy = RecoveryPolicy {
        version: 1,
        sequence: 1,
        threshold: 1,
        key_ids: vec![recovery_id],
        delay_blocks: 6,
        cancellation_rule: 1,
    };
    let policy_bytes = encode_recovery_policy(&policy);
    let bindings = |authorizer_role: u8, authorizer: [u8; 32], seal: [u8; 32]| SealBinding {
        authorizing_key_id: key_id(authorizer_role, authorizer),
        seal_xonly: seal,
    };
    let state_at = |sequence, previous_state, previous_seal, next_seal| ResultingState {
        sequence,
        previous_state,
        previous_seal,
        next_seal,
        controllers: vec![ControllerEntry {
            xonly: controller,
            capabilities: caps(),
        }],
        recovery: policy.clone(),
        controller_bindings: vec![bindings(1, controller, seal0)],
        recovery_bindings: vec![bindings(2, recovery, seal3)],
        lifecycle_status: 1,
    };
    let genesis_state = state_at(0, None, None, marker_outpoint(0x11, 0));
    let transition_state = state_at(
        1,
        Some(state),
        Some(marker_outpoint(0x11, 0)),
        marker_outpoint(0x12, 1),
    );
    let recovery_state = state_at(
        2,
        Some(state),
        Some(marker_outpoint(0x12, 1)),
        marker_outpoint(0x13, 2),
    );
    let signed = |name: &'static str,
                  object_type,
                  capability,
                  role,
                  signer_entity,
                  authorizing,
                  public,
                  mut body: Vec<u8>| {
        let mut payload = common_header(
            4,
            object_type,
            signer_entity,
            authorizing,
            key_id(role, public),
            role,
            capability,
        );
        payload.append(&mut body);
        (name, payload)
    };
    let mut cases = vec![
        signed("entity_genesis", 1, 1, 0, root_entity, None, root, {
            let mut body = 2u16.to_le_bytes().to_vec();
            body.extend_from_slice(&root);
            body.extend(encode_resulting_state(&genesis_state));
            body
        }),
        signed(
            "identity_transition",
            2,
            2,
            1,
            root_entity,
            Some(state),
            controller,
            {
                let mut body = vec![1];
                body.extend(encode_resulting_state(&transition_state));
                body
            },
        ),
        signed(
            "recovery_authorization",
            3,
            3,
            2,
            root_entity,
            Some(state),
            recovery,
            {
                let mut body = vec![3];
                body.extend(tagged_hash("O2A/v0.1/recovery-policy", &policy_bytes));
                body.extend(106u32.to_le_bytes());
                body.extend(encode_resulting_state(&recovery_state));
                body
            },
        ),
    ];
    let evidence_a = [0x10u8; 32];
    let evidence_b = [0x20u8; 32];
    let issued_at = 1_700_000_000u64;
    cases.push(signed(
        "attestation",
        5,
        5,
        1,
        root_entity,
        Some(state),
        controller,
        {
            let mut body = vec![1];
            body.extend(event_entity);
            body.extend(text_field("o2a.example/recognized-artist/v1").unwrap());
            body.extend(bytes_field(b"recognized"));
            body.extend(list_items(&[evidence_a.as_slice(), evidence_b.as_slice()]));
            body.extend(option_fixed(None));
            body.extend([0x31u8; 32]);
            body
        },
    ));
    cases.push(signed(
        "challenge",
        6,
        6,
        1,
        root_entity,
        Some(state),
        controller,
        {
            let mut body = [0x32u8; 32].to_vec();
            body.extend(text_field("o2a.example/dispute/v1").unwrap());
            body.extend(list_items(&[evidence_a.as_slice(), evidence_b.as_slice()]));
            body.extend(option_fixed(None));
            body.extend([0x33u8; 32]);
            body
        },
    ));
    cases.push(signed(
        "evidence_revocation",
        7,
        7,
        1,
        root_entity,
        Some(state),
        controller,
        {
            let mut body = [0x34u8; 32].to_vec();
            body.extend(text_field("o2a.example/withdrawn/v1").unwrap());
            body.extend(option_fixed(None));
            body.extend([0x35u8; 32]);
            body
        },
    ));
    let challenge_body = {
        let mut body = vec![1];
        body.extend(text_field("_o2a.example.test").unwrap());
        body.extend(1u16.to_le_bytes());
        body.extend([0x36u8; 32]);
        body.extend(issued_at.to_le_bytes());
        body.extend((issued_at + 3600).to_le_bytes());
        body.extend([0x37u8; 32]);
        body
    };
    let control = signed(
        "control_challenge",
        8,
        8,
        1,
        root_entity,
        Some(state),
        controller,
        challenge_body,
    );
    let challenge_id = tagged_hash("O2A/v0.1/control-challenge", &control.1);
    cases.push(control);
    cases.push(signed(
        "observation",
        9,
        9,
        1,
        root_entity,
        Some(state),
        controller,
        {
            let mut body = challenge_id.to_vec();
            body.push(1);
            body.extend(text_field("_o2a.example.test").unwrap());
            body.extend([0x38u8; 32]);
            body.extend((issued_at + 100).to_le_bytes());
            body.extend((issued_at + 3700).to_le_bytes());
            body.push(1);
            body.extend(option_fixed(None));
            body
        },
    ));
    cases.push(signed(
        "discovery_binding",
        10,
        10,
        1,
        root_entity,
        Some(state),
        controller,
        {
            let mut body = vec![1, 1];
            body.extend(bytes_field(&[0x71; 32]));
            body.extend(1u16.to_le_bytes());
            body.extend(issued_at.to_le_bytes());
            body.extend(option_fixed(Some(&(issued_at + 7200).to_le_bytes())));
            body.extend(option_fixed(Some(&[0x70; 32])));
            body.extend([0x39u8; 32]);
            body
        },
    ));
    cases.push(signed(
        "nostr_binding",
        10,
        10,
        1,
        root_entity,
        Some(state),
        controller,
        {
            let mut body = vec![2, 2];
            body.extend(bytes_field(&nostr));
            body.extend(1u16.to_le_bytes());
            body.extend(issued_at.to_le_bytes());
            body.extend(option_fixed(None));
            body.extend(option_fixed(Some(&[0x72; 32])));
            body.extend([0x73u8; 32]);
            body
        },
    ));
    let artist = {
        let mut row = root_entity.to_vec();
        row.extend(text_field("artist/v1").unwrap());
        row
    };
    let venue = {
        let mut row = event_entity.to_vec();
        row.extend(text_field("venue/v1").unwrap());
        row
    };
    let mut participants = vec![artist.clone(), venue];
    participants.sort();
    cases.push(signed(
        "event_manifest",
        11,
        11,
        1,
        event_entity,
        Some(next_state),
        controller,
        {
            let mut body = vec![1];
            body.extend(1u32.to_le_bytes());
            body.extend([0x81u8; 32]);
            body.extend(list_items(&[[0x82u8; 32].as_slice()]));
            body.extend(list_items(&[[0x83u8; 32].as_slice()]));
            body.extend(list_items(&participants));
            body.extend(option_fixed(None));
            body.extend(list_items(&[content_reference(
                "application/vnd.o2a.event+json",
                128,
                [0x84; 32],
            )
            .unwrap()]));
            body.extend(root_entity);
            body.extend(option_fixed(None));
            body
        },
    ));
    cases.push(signed(
        "album_manifest",
        11,
        11,
        1,
        album_entity,
        Some([0x24; 32]),
        controller,
        {
            let mut body = vec![2];
            body.extend(1u32.to_le_bytes());
            body.extend([0x91u8; 32]);
            body.extend(list_items(&[] as &[&[u8]]));
            body.extend(list_items(&[] as &[&[u8]]));
            body.extend(list_items(&[artist.as_slice()]));
            body.extend(option_fixed(Some(&[0x92; 32])));
            body.extend(list_items(&[content_reference(
                "audio/flac",
                4096,
                [0x93; 32],
            )
            .unwrap()]));
            body.extend(root_entity);
            body.extend(option_fixed(None));
            body
        },
    ));
    cases
}

#[test]
fn protocol_object_bytes_match_the_authority_commit() {
    let fixture = git_show("tests/vectors/protocol-objects-v0.1.json");
    let json = parse_json(&fixture);
    let cases = json.obj("cases");
    for (name, payload) in protocol_payloads() {
        let case = cases.obj(name);
        let expected = case.string("payload_hex");
        assert_eq!(hex::encode(&payload), expected, "{name} payload");
        assert_eq!(
            payload.len(),
            case.number("payload_length") as usize,
            "{name} length"
        );
        assert_eq!(
            hex::encode(tagged_hash(case.string("tag"), &payload)),
            case.string("digest_hex"),
            "{name} digest"
        );
        let signature = hx32_pair(case.string("signature_hex"));
        let public = hx32(case.string("public_key"));
        assert!(
            signature_accepts(case.string("tag"), &payload, signature, public),
            "{name} signature"
        );
        let mut mutated = signature;
        mutated[63] ^= 0xff;
        assert!(
            !signature_accepts(case.string("tag"), &payload, mutated, public),
            "{name} mutated signature"
        );
        let other = if case.string("tag") == "O2A/v0.1/attestation" {
            "O2A/v0.1/claim"
        } else {
            "O2A/v0.1/attestation"
        };
        assert!(
            !signature_accepts(other, &payload, signature, public),
            "{name} cross-domain"
        );
    }
}

fn hx32_pair(value: &str) -> [u8; 64] {
    hx(value).try_into().expect("64-byte signature")
}

#[test]
fn seal_script_bytes_match_the_authority_commit() {
    let fixture = git_show("tests/vectors/seal-script-v0.1.json");
    let core = git_show("tests/vectors/seal-script-core-v0.1.json");
    let parsed = parse_json(&fixture);
    let parsed_core = parse_json(&core);
    let cases = parsed.obj("cases");
    let core_cases = parsed_core.obj("cases");
    for name in ["one_controller", "two_controllers_odd", "three_controllers"] {
        let case = cases.obj(name);
        let controllers = bindings(case, "controller_seal_bindings", "controller_key_id");
        let recovery = bindings(case, "recovery_seal_bindings", "recovery_key_id");
        let policy = encode_seal_policy(&controllers, &recovery);
        let built = seal_script(
            &controllers,
            &recovery,
            case.number("threshold"),
            case.number("delay_blocks"),
            policy,
        )
        .unwrap_or_else(|err| panic!("{name}: {err}"));
        let expected = case.obj("expected");
        assert_eq!(
            hex::encode(&built.policy),
            expected.string("policy_hex"),
            "{name} policy"
        );
        let scripts = expected.array("scripts_hex");
        assert_eq!(built.scripts.len(), scripts.len(), "{name} script count");
        for (actual, expected_script) in built.scripts.iter().zip(scripts) {
            assert_eq!(
                hex::encode(actual),
                expected_script.string_value(),
                "{name} script"
            );
        }
        for (actual, expected_leaf) in built.leaf_hashes.iter().zip(expected.array("leaf_hashes")) {
            assert_eq!(
                hex::encode(actual),
                expected_leaf.string_value(),
                "{name} leaf"
            );
        }
        assert_eq!(
            hex::encode(built.merkle_root),
            expected.string("merkle_root"),
            "{name} root"
        );
        assert_eq!(
            hex::encode(built.output_key),
            expected.string("output_key"),
            "{name} output"
        );
        assert_eq!(
            hex::encode(&built.script_pubkey),
            expected.string("script_pubkey"),
            "{name} spk"
        );
        assert_eq!(
            hex::encode(&built.script_pubkey),
            core_cases.obj(name).string("script_pubkey"),
            "{name} core spk"
        );
    }
}

fn bindings(case: &Json, field: &str, id_field: &str) -> Vec<SealBinding> {
    case.array(field)
        .iter()
        .map(|binding| SealBinding {
            authorizing_key_id: hx32(binding.string(id_field)),
            seal_xonly: hx32(binding.string("seal_xonly")),
        })
        .collect()
}

#[test]
fn role_four_derivation_and_rejections_match_the_authority_commit() {
    let fixture = git_show("tests/vectors/derivation-v0.1.json");
    let json = parse_json(&fixture);
    let seed = json.obj("seed").string("seed_hex");
    assert_eq!(seed, UNSAFE_BIP39_SEED_HEX);
    for positive in json.array("positives") {
        let coin = positive.number("coin_type") as u32;
        let entity = positive.number("entity") as u32;
        let keys = positive.obj("keys");
        for name in ["seal_0", "seal_1", "seal_2", "seal_3", "seal_4", "seal_5"] {
            let entry = keys.obj(name);
            let index = name.rsplit('_').next().unwrap().parse::<u32>().unwrap();
            let derived = identity_key(coin, entity, 4, index);
            assert_eq!(hex::encode(derived.xonly), entry.string("xonly"), "{name}");
            assert_eq!(
                entry.string("path"),
                format!("m/{coin}'/{entity}'/4'/{index}'")
            );
        }
    }
}

/// Every `rejections` id, every `expected_result` that says reject, and every
/// `rejected_*` field in the authority vector JSON. A new id fails this test
/// until it is added here.
const JSON_REJECTS: &[(&str, &str)] = &[
    (
        "cross-entity-reuse",
        "entity 0 key cannot fill entity 1 path",
    ),
    ("cross-role-reuse", "root key cannot fill controller role"),
    ("path-alias", "root terminal index is fixed at zero"),
    (
        "payment-key-o2a-signature",
        "payment keys have no O2A signing role",
    ),
    ("rejected_object_length", "oversized bytes field"),
    ("rejected_text_length", "oversized text field"),
    (
        "retired-827-path",
        "retired purpose path is outside Route B",
    ),
    ("root_validation", "root is not a BIP340 x-only public key"),
    ("unhardened-index", "identity key index must be hardened"),
    ("unhardened-role", "identity role must be hardened"),
    ("unknown_network", "unknown Bitcoin network"),
    (
        "unsupported-profile-version",
        "unknown derivation profile version",
    ),
    (
        "wrong-coin-type",
        "coin type does not match declared network",
    ),
    ("wrong_capability", "object/domain/capability mismatch"),
    ("wrong_network", "wrong verifier network"),
    ("wrong_role", "wrong signing-key role"),
];

#[test]
fn recovery_policy_hash_matches_the_authority_vector() {
    let recovery = hx32("dd308afec5777e13121fa72b9cc1b7cc0139715309b086c960e18fd969774eb8");
    let policy = RecoveryPolicy {
        version: 1,
        sequence: 1,
        threshold: 1,
        key_ids: vec![key_id(2, recovery)],
        delay_blocks: 6,
        cancellation_rule: 1,
    };
    let encoded = encode_recovery_policy(&policy);
    let hashed = recovery_policy_hash(&policy);
    assert_eq!(hashed, tagged_hash("O2A/v0.1/recovery-policy", &encoded));
    let fixture = parse_json(&git_show("tests/vectors/protocol-objects-v0.1.json"));
    let payload = hx(fixture
        .obj("cases")
        .obj("recovery_authorization")
        .string("payload_hex"));
    assert_eq!(payload.get(105), Some(&3), "recovery operation");
    assert_eq!(&payload[106..138], &hashed);
}

#[test]
fn every_json_reject_case_maps_to_one_reason() {
    let mut discovered = Vec::new();
    for path in vector_json_paths() {
        let parsed = parse_json(&git_show(&path));
        collect_reject_ids(&parsed, None, &mut discovered);
    }
    discovered.sort();
    let mut unique = discovered.clone();
    unique.dedup();
    assert_eq!(discovered, unique, "duplicate reject id");
    let mapped = JSON_REJECTS.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let mut mapped_sorted = mapped.clone();
    mapped_sorted.sort();
    assert_eq!(mapped_sorted, mapped, "JSON_REJECTS must stay sorted by id");
    assert_eq!(
        unique, mapped,
        "an authority reject case is unmapped, or the map names a case the vectors do not have"
    );
    for (id, reason) in JSON_REJECTS {
        assert_eq!(json_reject_reason(id), *reason, "{id}");
    }
}

fn json_reject_reason(id: &str) -> &'static str {
    let derivation = parse_json(&git_show("tests/vectors/derivation-v0.1.json"));
    if let Some(rejection) = derivation
        .array("rejections")
        .iter()
        .find(|item| item.string("id") == id)
    {
        let candidate = hx32(rejection.string("candidate_xonly"));
        let error = accept_identity_key(
            rejection.number("profile_version") as u16,
            rejection.string("network"),
            rejection.string("path"),
            &candidate,
        )
        .expect_err(id);
        assert_eq!(error, rejection.string("reason"), "{id} vector reason");
        return error;
    }
    let vectors = parse_json(&git_show("tests/vectors/v0.1.json"));
    let semantic = vectors.obj("semantic_claims");
    let public_key = hx32(vectors.obj("public_test_key").string("xonly_hex"));
    let claim = vectors.obj("claim");
    let primary = OwnedAuth::from_json(semantic.obj("primary_authorization"));
    let primary_payload = hx(claim.string("payload_hex"));
    let primary_signature = hx32_pair(claim.string("signature_hex"));
    evaluate_name_claim(
        &primary_payload,
        primary_signature,
        public_key,
        semantic.number("expected_network") as u8,
        &primary.view(),
    )
    .expect("primary name claim");
    match id {
        "root_validation" => entity_id_checked(
            semantic.number("expected_network") as u8,
            &hx32(semantic.obj("root_validation").string("invalid_xonly_hex")),
        )
        .expect_err(id),
        "rejected_object_length" | "rejected_text_length" => {
            let needle = if id == "rejected_text_length" {
                claim.string("predicate_utf8").as_bytes()
            } else {
                claim.string("object_utf8").as_bytes()
            };
            let mut payload = primary_payload;
            let at = payload
                .windows(needle.len())
                .position(|window| window == needle)
                .expect("claim field");
            let length_at = at - 4;
            let length = semantic.obj("decoder_bounds").number(id) as u32;
            payload[length_at..length_at + 4].copy_from_slice(&length.to_le_bytes());
            parse_claim(&payload).expect_err(id)
        }
        "wrong_capability" | "wrong_role" | "unknown_network" => {
            let case = semantic.obj(id);
            let expected = if id == "unknown_network" {
                case.number("network") as u8
            } else {
                semantic.number("expected_network") as u8
            };
            evaluate_name_claim(
                &hx(case.string("payload_hex")),
                hx32_pair(case.string("signature_hex")),
                public_key,
                expected,
                &primary.view(),
            )
            .expect_err(id)
        }
        "wrong_network" => {
            let case = semantic.obj(id);
            let payload = hx(case.string("payload_hex"));
            let signature = hx32_pair(case.string("signature_hex"));
            let authorization = OwnedAuth::from_json(case.obj("authorization"));
            let rejected = evaluate_name_claim(
                &payload,
                signature,
                public_key,
                semantic.number("expected_network") as u8,
                &authorization.view(),
            )
            .expect_err(id);
            evaluate_name_claim(
                &payload,
                signature,
                public_key,
                case.number("network") as u8,
                &authorization.view(),
            )
            .expect("the same claim is valid in its own network context");
            rejected
        }
        other => panic!("unmapped json reject {other}"),
    }
}

struct OwnedAuth {
    entity: [u8; 32],
    state: Option<[u8; 32]>,
    key_id: [u8; 32],
    public_key: [u8; 32],
    key_role: u8,
    capabilities: Vec<u16>,
}

impl OwnedAuth {
    fn from_json(value: &Json) -> Self {
        Self {
            entity: hx32(value.string("entity")),
            state: Some(hx32(value.string("state"))),
            key_id: hx32(value.string("key_id")),
            public_key: hx32(value.string("public_key")),
            key_role: value.number("key_role") as u8,
            capabilities: value
                .array("capabilities")
                .iter()
                .map(|item| item.number_value() as u16)
                .collect(),
        }
    }

    fn view(&self) -> ClaimAuthorization<'_> {
        ClaimAuthorization {
            entity: &self.entity,
            state: self.state.as_ref(),
            key_id: &self.key_id,
            public_key: &self.public_key,
            key_role: self.key_role,
            capabilities: &self.capabilities,
        }
    }
}

fn vector_json_paths() -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(protocol_repo())
        .args(["ls-tree", "-r", "--name-only", SPEC_COMMIT, "tests/vectors"])
        .output()
        .expect("git ls-tree");
    assert!(
        output.status.success(),
        "vector list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("vector list utf-8")
        .lines()
        .filter(|line| line.ends_with(".json"))
        .map(str::to_string)
        .collect()
}

fn collect_reject_ids(value: &Json, key: Option<&str>, out: &mut Vec<String>) {
    match value {
        Json::Object(fields) => {
            if let Some(name) = key {
                if let Some((_, Json::String(expected))) =
                    fields.iter().find(|(field, _)| field == "expected_result")
                {
                    if expected.contains("reject") {
                        out.push(name.to_string());
                    }
                }
            }
            for (name, child) in fields {
                if name == "rejections" {
                    if let Json::Array(items) = child {
                        for item in items {
                            out.push(item.string("id").to_string());
                        }
                    }
                }
                if name.starts_with("rejected_") {
                    out.push(name.clone());
                }
                collect_reject_ids(child, Some(name), out);
            }
        }
        Json::Array(items) => {
            for item in items {
                collect_reject_ids(item, None, out);
            }
        }
        _ => {}
    }
}

/// Adversarial cases that live in the Python checkers rather than a JSON
/// `rejections` array. Each row names the checker function it mirrors.
const CHECKER_REJECTS: &[(&str, &str, &str)] = &[
    (
        "delay-0",
        "check_protocol_objects.py:valid_recovery_policy",
        "delay_blocks is outside 1..=65535",
    ),
    (
        "delay-65536",
        "check_protocol_objects.py:valid_recovery_policy",
        "delay_blocks is outside 1..=65535",
    ),
    (
        "threshold-above-set",
        "check_protocol_objects.py:valid_recovery_policy",
        "recovery threshold is outside the key set",
    ),
    (
        "unknown-seal-policy-version",
        "check_protocol_objects.py:valid_seal_policy",
        "unknown seal-policy version",
    ),
    (
        "unsorted-controller-bindings",
        "check_protocol_objects.py:valid_seal_policy",
        "unsorted controller seal bindings",
    ),
    (
        "unsorted-recovery-bindings",
        "check_protocol_objects.py:valid_seal_policy",
        "unsorted recovery seal bindings",
    ),
    (
        "duplicate-recovery-seal-key",
        "check_protocol_objects.py:valid_seal_policy",
        "duplicate seal key",
    ),
    (
        "stale-controller-binding",
        "check_protocol_objects.py:valid_seal_policy",
        "controller seal bindings do not cover the transition key set",
    ),
    (
        "unpaired-controller-key",
        "check_protocol_objects.py:valid_seal_policy",
        "controller seal bindings do not cover the transition key set",
    ),
    (
        "unpaired-recovery-key",
        "check_protocol_objects.py:valid_seal_policy",
        "recovery seal bindings do not cover the recovery key set",
    ),
    (
        "seal-key-reused-as-controller",
        "check_protocol_objects.py:valid_seal_policy",
        "cross-role x-only reuse",
    ),
    (
        "mismatched-seal-script",
        "check_protocol_objects.py:seal_output_matches",
        "seal scriptPubKey mismatch",
    ),
    (
        "history-open-unspent",
        "check_protocol_objects.py:identity_history_outcome",
        "CURRENT",
    ),
    (
        "history-missing-observation",
        "check_protocol_objects.py:identity_history_outcome",
        "INCOMPLETE",
    ),
    (
        "history-unproven-spend",
        "check_protocol_objects.py:identity_history_outcome",
        "INCOMPLETE",
    ),
    (
        "history-under-depth",
        "check_protocol_objects.py:identity_history_outcome",
        "INCOMPLETE",
    ),
    (
        "history-valid-successor",
        "check_protocol_objects.py:identity_history_outcome",
        "CURRENT",
    ),
    (
        "history-closed-without-transition",
        "check_protocol_objects.py:identity_history_outcome",
        "SEAL_CLOSED_WITHOUT_VALID_TRANSITION",
    ),
    (
        "role-bound-key-ids",
        "check_protocol_objects.py:check_seal_vectors",
        "role-bound key ids differ",
    ),
    (
        "genesis-wrong-root",
        "check_protocol_objects.py:check",
        "genesis root does not match the signing key",
    ),
    (
        "transition-operation-2",
        "check_protocol_objects.py:check",
        "unsupported in demo",
    ),
    (
        "transition-operation-3",
        "check_protocol_objects.py:check",
        "operation 3 is invalid in the identity-transition domain",
    ),
    (
        "transition-operation-4",
        "check_protocol_objects.py:check",
        "unsupported in demo",
    ),
    (
        "unknown-state-capability",
        "check_protocol_objects.py:valid_state",
        "unknown capability",
    ),
    (
        "recovery-too-early",
        "check_protocol_objects.py:evaluate_signed_payload",
        "too_early",
    ),
    (
        "recovery-incomplete-threshold",
        "check_protocol_objects.py:evaluate_signed_payload",
        "incomplete",
    ),
    (
        "recovery-wrong-policy-hash",
        "check_protocol_objects.py:evaluate_signed_payload",
        "recovery policy hash does not match the prior state",
    ),
    (
        "recovery-anchor-not-before",
        "check_protocol_objects.py:evaluate_signed_payload",
        "not_before_height does not match seal creation plus delay",
    ),
    (
        "attestation-duplicate-evidence",
        "check_protocol_objects.py:evaluate_signed_payload",
        "duplicate evidence",
    ),
    (
        "challenge-reversed-evidence",
        "check_protocol_objects.py:evaluate_signed_payload",
        "unsorted evidence",
    ),
    (
        "revocation-entity-target",
        "check_protocol_objects.py:evaluate_signed_payload",
        "evidence revocation target is an entity id",
    ),
    (
        "control-non-increasing-expiry",
        "check_protocol_objects.py:evaluate_signed_payload",
        "expiry does not increase",
    ),
    (
        "observation-not-yet",
        "check_protocol_objects.py:evaluate_signed_payload",
        "not_yet_observed",
    ),
    (
        "observation-expired",
        "check_protocol_objects.py:evaluate_signed_payload",
        "expired",
    ),
    (
        "observation-invalid-window",
        "check_protocol_objects.py:evaluate_signed_payload",
        "expiry does not increase",
    ),
    (
        "discovery-wrong-scheme",
        "check_protocol_objects.py:evaluate_signed_payload",
        "unknown adapter key scheme",
    ),
    (
        "nostr-wrong-scheme",
        "check_protocol_objects.py:evaluate_signed_payload",
        "unknown adapter key scheme",
    ),
    (
        "nostr-adapter-reuses-root",
        "check_protocol_objects.py:check",
        "adapter key reuses the issuer root",
    ),
    (
        "package-omitted-object",
        "check_protocol_objects.py:package_object_result",
        "incomplete",
    ),
    (
        "package-unnamed-object",
        "check_protocol_objects.py:package_object_result",
        "incomplete",
    ),
    (
        "event-wrong-entity",
        "check_protocol_objects.py:evaluate_signed_payload",
        "manifest signer entity mismatch",
    ),
    (
        "album-missing-track",
        "check_protocol_objects.py:evaluate_signed_payload",
        "album manifest omits its track manifest",
    ),
    (
        "claim-truncated",
        "check_vectors.py:evaluate_name_claim",
        "truncated claim",
    ),
    (
        "claim-missing-authorizing-state",
        "check_vectors.py:evaluate_name_claim",
        "missing authorizing state",
    ),
    (
        "claim-arbitrary-key-id",
        "check_vectors.py:evaluate_name_claim",
        "signing key ID does not match role and public key",
    ),
];

#[test]
fn checker_reject_cases_cite_their_source_function() {
    let payloads = protocol_payloads();
    assert_eq!(payloads.len(), 12, "protocol payload inventory changed");
    for (name, payload) in &payloads {
        decode_payload(payload).unwrap_or_else(|err| panic!("{name}: {err}"));
        let mut short = payload.clone();
        short.pop();
        assert_eq!(
            decode_payload(&short),
            Err("truncated payload"),
            "{name}-truncated mirrors decode_payload"
        );
        let mut extra = payload.clone();
        extra.push(0);
        assert_eq!(
            decode_payload(&extra),
            Err("trailing payload bytes"),
            "{name}-trailing-byte mirrors decode_payload"
        );
        if *name != "entity_genesis" {
            assert!(payload.len() > 105, "{name} header");
            let capability = u16::from_le_bytes(payload[103..105].try_into().unwrap());
            assert_eq!(
                state_authorizes(capability, &[]),
                Err("missing state capability"),
                "{name}-missing-capability mirrors evaluate_signed_payload"
            );
        }
    }
    for (id, _source, reason) in CHECKER_REJECTS {
        assert_eq!(checker_reason(id), *reason, "{id}");
    }
}

fn checker_reason(id: &str) -> &'static str {
    let (controllers, recovery) = seal_case("one_controller");
    let controller_ids = binding_ids(&controllers);
    let recovery_ids = binding_ids(&recovery);
    let (two_controllers, _) = seal_case("two_controllers_odd");
    let two_ids = binding_ids(&two_controllers);
    let err = |result: Result<(), &'static str>| result.expect_err(id);
    let clock = |not_before, policy, prior, threshold, block| RecoveryClock {
        not_before_height: not_before,
        policy_hash: [policy; 32],
        prior_policy_hash: [prior; 32],
        prior_threshold: threshold,
        seal_creation_height: 100,
        delay_blocks: 6,
        block_height: block,
        signer_in_recovery_set: true,
    };
    match id {
        "delay-0" => err(recovery_policy_valid(&sample_policy(0, 2))),
        "delay-65536" => err(recovery_policy_valid(&sample_policy(65_536, 2))),
        "threshold-above-set" => err(recovery_policy_valid(&sample_policy(144, 4))),
        "unknown-seal-policy-version" => err(seal_policy_valid(
            2,
            &controllers,
            &recovery,
            &controller_ids,
            &recovery_ids,
            &[],
        )),
        "unsorted-controller-bindings" => {
            let mut reversed = two_controllers;
            reversed.reverse();
            err(seal_policy_valid(
                1,
                &reversed,
                &recovery,
                &two_ids,
                &recovery_ids,
                &[],
            ))
        }
        "unsorted-recovery-bindings" => {
            let mut reversed = recovery;
            reversed.reverse();
            err(seal_policy_valid(
                1,
                &controllers,
                &reversed,
                &controller_ids,
                &recovery_ids,
                &[],
            ))
        }
        "duplicate-recovery-seal-key" => {
            let mut duplicate = recovery;
            duplicate[1].seal_xonly = duplicate[0].seal_xonly;
            duplicate.sort_by(|left, right| left.bytes().cmp(&right.bytes()));
            err(seal_policy_valid(
                1,
                &controllers,
                &duplicate,
                &controller_ids,
                &recovery_ids,
                &[],
            ))
        }
        "stale-controller-binding" => err(seal_policy_valid(
            1,
            &[SealBinding {
                authorizing_key_id: [0xff; 32],
                seal_xonly: controllers[0].seal_xonly,
            }],
            &recovery,
            &controller_ids,
            &recovery_ids,
            &[],
        )),
        "unpaired-controller-key" => {
            let mut expected = controller_ids.clone();
            expected.push([0xee; 32]);
            err(seal_policy_valid(
                1,
                &controllers,
                &recovery,
                &expected,
                &recovery_ids,
                &[],
            ))
        }
        "unpaired-recovery-key" => err(seal_policy_valid(
            1,
            &controllers,
            &recovery[..recovery.len() - 1],
            &controller_ids,
            &recovery_ids,
            &[],
        )),
        "seal-key-reused-as-controller" => err(seal_policy_valid(
            1,
            &controllers,
            &recovery,
            &controller_ids,
            &recovery_ids,
            &[controllers[0].seal_xonly],
        )),
        "mismatched-seal-script" => {
            let parsed = parse_json(&git_show("tests/vectors/seal-script-v0.1.json"));
            let script = hx(parsed
                .obj("cases")
                .obj("one_controller")
                .obj("expected")
                .string("script_pubkey"));
            let mut mismatched = script.clone();
            let last = mismatched.len() - 1;
            mismatched[last] ^= 1;
            err(seal_output_matches(&script, &mismatched))
        }
        "history-open-unspent" => history(true, Some(SealWatch::Unspent), false, 0, false),
        "history-missing-observation" => history(false, None, false, 0, false),
        "history-unproven-spend" => history(true, Some(SealWatch::Spent), false, 1, false),
        "history-under-depth" => history(true, Some(SealWatch::Spent), true, 0, false),
        "history-valid-successor" => history(true, Some(SealWatch::Spent), true, 1, true),
        "history-closed-without-transition" => {
            history(true, Some(SealWatch::Spent), true, 1, false)
        }
        "role-bound-key-ids" => {
            assert_ne!(
                key_id(4, controllers[0].seal_xonly),
                key_id(1, controllers[0].seal_xonly)
            );
            "role-bound key ids differ"
        }
        "genesis-wrong-root" => err(genesis_root_ok(&[0x11; 32], &[0x22; 32])),
        "transition-operation-2" => transition_operation_reason(2),
        "transition-operation-3" => transition_operation_reason(3),
        "transition-operation-4" => transition_operation_reason(4),
        "unknown-state-capability" => err(capability_known(13)),
        "recovery-too-early" => {
            assert_eq!(
                recovery_witness_status(&clock(106, 9, 9, 1, 106)),
                Ok("valid")
            );
            recovery_witness_status(&clock(106, 9, 9, 1, 105)).expect_err(id)
        }
        "recovery-incomplete-threshold" => {
            // signature_count is intentionally absent. The checker passes 2
            // and still receives incomplete.
            recovery_witness_status(&clock(106, 9, 9, 2, 106)).expect_err(id)
        }
        "recovery-wrong-policy-hash" => {
            recovery_witness_status(&clock(106, 1, 9, 1, 106)).expect_err(id)
        }
        "recovery-anchor-not-before" => {
            recovery_witness_status(&clock(96, 9, 9, 1, 106)).expect_err(id)
        }
        "attestation-duplicate-evidence" => err(evidence_ids_valid(&[[0x10; 32], [0x10; 32]])),
        "challenge-reversed-evidence" => err(evidence_ids_valid(&[[0x20; 32], [0x10; 32]])),
        "revocation-entity-target" => err(revocation_target(&[0x34; 32], &[[0x34; 32]])),
        "control-non-increasing-expiry" => err(increasing_expiry(1_700_000_000, 1_700_000_000)),
        "observation-not-yet" => {
            observation_time_status(1_700_000_100, 1_700_003_700, Some(1_700_000_099))
                .expect_err(id)
        }
        "observation-expired" => {
            assert_eq!(
                observation_time_status(1_700_000_100, 1_700_003_700, Some(1_700_003_600)),
                Ok("valid")
            );
            observation_time_status(1_700_000_100, 1_700_003_700, Some(1_700_003_701))
                .expect_err(id)
        }
        "observation-invalid-window" => err(increasing_expiry(1_700_000_100, 1_700_000_100)),
        "discovery-wrong-scheme" => err(adapter_scheme(1, 2)),
        "nostr-wrong-scheme" => err(adapter_scheme(2, 1)),
        "nostr-adapter-reuses-root" => err(adapter_key_distinct(&[0x44; 32], &[0x44; 32])),
        "package-omitted-object" | "package-unnamed-object" => package_object_gap(false),
        "event-wrong-entity" => err(manifest_binding(1, &[0x11; 32], &[0x22; 32], false)),
        "album-missing-track" => err(manifest_binding(2, &[0x11; 32], &[0x11; 32], false)),
        "claim-truncated" | "claim-missing-authorizing-state" | "claim-arbitrary-key-id" => {
            claim_checker_reason(id)
        }
        other => panic!("unmapped checker case {other}"),
    }
}

fn sample_policy(delay_blocks: u32, threshold: u16) -> RecoveryPolicy {
    RecoveryPolicy {
        version: 1,
        sequence: 1,
        threshold,
        key_ids: vec![[1; 32], [2; 32], [3; 32]],
        delay_blocks,
        cancellation_rule: 1,
    }
}

fn seal_case(name: &str) -> (Vec<SealBinding>, Vec<SealBinding>) {
    let parsed = parse_json(&git_show("tests/vectors/seal-script-v0.1.json"));
    let case = parsed.obj("cases").obj(name);
    (
        bindings(&case, "controller_seal_bindings", "controller_key_id"),
        bindings(&case, "recovery_seal_bindings", "recovery_key_id"),
    )
}

fn binding_ids(bindings: &[SealBinding]) -> Vec<[u8; 32]> {
    bindings
        .iter()
        .map(|binding| binding.authorizing_key_id)
        .collect()
}

fn history(
    view: bool,
    seal: Option<SealWatch>,
    spend_proof: bool,
    spend_confirmations: u32,
    valid_transition: bool,
) -> &'static str {
    identity_history_state(&HistoryInput {
        has_bitcoin_view: view,
        has_best_block: view,
        observed_height: view.then_some(120),
        seal,
        spend_proof,
        spend_confirmations,
        required_depth: 1,
        valid_transition,
    })
}

fn transition_operation_reason(operation: u8) -> &'static str {
    use crate::{
        controller_rotation, demo_keys, header_end, sign, verify_controller_rotation,
        TRANSITION_TAG,
    };
    let base = controller_rotation([2; 32], [1; 36], [3; 36]);
    let end = header_end(&base.payload).expect("header");
    let mut payload = base.payload;
    payload[end] = operation;
    let signed = sign(TRANSITION_TAG, payload, demo_keys().controller_0);
    verify_controller_rotation(&signed).expect_err("operation")
}

fn claim_checker_reason(id: &str) -> &'static str {
    use secp256k1::{Keypair, Message, Secp256k1, SecretKey};
    let vectors = parse_json(&git_show("tests/vectors/v0.1.json"));
    let claim = vectors.obj("claim");
    let payload = hx(claim.string("payload_hex"));
    let public_key = hx32(vectors.obj("public_test_key").string("xonly_hex"));
    let primary = OwnedAuth::from_json(vectors.obj("semantic_claims").obj("primary_authorization"));
    if id == "claim-truncated" {
        return evaluate_name_claim(
            &payload[..payload.len() - 1],
            [0; 64],
            public_key,
            4,
            &primary.view(),
        )
        .expect_err(id);
    }
    let mut edited = payload;
    if id == "claim-missing-authorizing-state" {
        let marker = 2 + 1 + 2 + 32;
        assert_eq!(edited[marker], 1);
        edited = [&edited[..marker], &[0], &edited[marker + 33..]].concat();
    } else {
        let start = 2 + 1 + 2 + 32 + 1 + 32;
        edited[start..start + 32].fill(0xa5);
    }
    let mut scalar = [0u8; 32];
    scalar[31] = 3;
    let secret = SecretKey::from_slice(&scalar).expect("published scalar 3");
    let secp = Secp256k1::new();
    let pair = Keypair::from_secret_key(&secp, &secret);
    let digest = tagged_hash("O2A/v0.1/claim", &edited);
    let signature = secp.sign_schnorr_no_aux_rand(&Message::from_digest(digest), &pair);
    evaluate_name_claim(&edited, *signature.as_ref(), public_key, 4, &primary.view()).expect_err(id)
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum Json {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn obj(&self, key: &str) -> &Json {
        match self {
            Json::Object(fields) => {
                for (name, value) in fields {
                    if name == key {
                        return value;
                    }
                }
                panic!("missing {key}");
            }
            _ => panic!("{key} is not an object field"),
        }
    }

    fn string(&self, key: &str) -> &str {
        match self.obj(key) {
            Json::String(value) => value,
            _ => panic!("{key} is not a string"),
        }
    }

    fn number(&self, key: &str) -> u64 {
        match self.obj(key) {
            Json::Number(value) => *value,
            _ => panic!("{key} is not a number"),
        }
    }

    fn array(&self, key: &str) -> &[Json] {
        match self.obj(key) {
            Json::Array(value) => value,
            _ => panic!("{key} is not an array"),
        }
    }

    fn string_value(&self) -> &str {
        match self {
            Json::String(value) => value,
            _ => panic!("expected string"),
        }
    }

    fn number_value(&self) -> u64 {
        match self {
            Json::Number(value) => *value,
            _ => panic!("expected number"),
        }
    }
}

fn parse_json(input: &str) -> Json {
    let mut parser = Parser { input, index: 0 };
    let value = parser.value();
    parser.skip();
    assert!(parser.done(), "trailing json");
    value
}

struct Parser<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> Parser<'a> {
    fn done(&self) -> bool {
        self.index >= self.input.len()
    }

    fn skip(&mut self) {
        while self.input[self.index..].starts_with(|ch: char| ch.is_whitespace()) {
            self.index += 1;
        }
    }

    fn raw(&self) -> char {
        self.input[self.index..]
            .chars()
            .next()
            .expect("unexpected end")
    }

    fn peek(&mut self) -> char {
        self.skip();
        self.raw()
    }

    fn bump(&mut self) -> char {
        let ch = self.peek();
        self.index += ch.len_utf8();
        ch
    }

    fn bump_raw(&mut self) -> char {
        let ch = self.raw();
        self.index += ch.len_utf8();
        ch
    }

    fn value(&mut self) -> Json {
        match self.peek() {
            '{' => self.object(),
            '[' => self.array(),
            '"' => Json::String(self.string()),
            't' => {
                self.eat("true");
                Json::Bool(true)
            }
            'f' => {
                self.eat("false");
                Json::Bool(false)
            }
            'n' => {
                self.eat("null");
                Json::Null
            }
            '-' | '0'..='9' => self.number(),
            other => panic!("unexpected json {other}"),
        }
    }

    fn eat(&mut self, expected: &str) {
        assert!(
            self.input[self.index..].starts_with(expected),
            "expected {expected}"
        );
        self.index += expected.len();
    }

    fn object(&mut self) -> Json {
        self.bump();
        let mut fields = Vec::new();
        if self.peek() == '}' {
            self.bump();
            return Json::Object(fields);
        }
        loop {
            let key = self.string();
            assert_eq!(self.bump(), ':');
            fields.push((key, self.value()));
            match self.bump() {
                ',' => continue,
                '}' => break,
                other => panic!("object {other}"),
            }
        }
        Json::Object(fields)
    }

    fn array(&mut self) -> Json {
        self.bump();
        let mut items = Vec::new();
        if self.peek() == ']' {
            self.bump();
            return Json::Array(items);
        }
        loop {
            items.push(self.value());
            match self.bump() {
                ',' => continue,
                ']' => break,
                other => panic!("array {other}"),
            }
        }
        Json::Array(items)
    }

    fn string(&mut self) -> String {
        assert_eq!(self.bump(), '"');
        let mut out = String::new();
        loop {
            match self.bump_raw() {
                '"' => break,
                '\\' => {
                    let escaped = self.bump();
                    out.push(match escaped {
                        '"' | '\\' | '/' => escaped,
                        'n' => '\n',
                        't' => '\t',
                        'u' => {
                            let hex = &self.input[self.index..self.index + 4];
                            self.index += 4;
                            char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()
                        }
                        other => panic!("escape {other}"),
                    });
                }
                ch => out.push(ch),
            }
        }
        out
    }

    fn number(&mut self) -> Json {
        self.skip();
        let start = self.index;
        if self.peek() == '-' {
            self.bump();
        }
        while self.index < self.input.len()
            && self.input[self.index..].starts_with(|ch: char| ch.is_ascii_digit())
        {
            self.index += 1;
        }
        let text = &self.input[start..self.index];
        Json::Number(text.parse().unwrap_or_else(|_| panic!("number {text}")))
    }
}
