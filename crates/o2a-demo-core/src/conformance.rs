//! Byte-for-byte checks against the pinned sibling specification commit.
//!
//! The vectors are read with `git show <SPEC_COMMIT>:path`. A missing sibling
//! repository or commit fails the test.

use std::path::PathBuf;
use std::process::Command;

use crate::encode::{
    bytes_field, common_header, content_reference, encode_recovery_policy, encode_resulting_state,
    encode_seal_policy, key_id, list_items, option_fixed, text_field, ControllerEntry,
    RecoveryPolicy, ResultingState, SealBinding,
};
use crate::seal::seal_script;
use crate::{
    accept_identity_key, entity_id, identity_key, signature_accepts, tagged_hash, SPEC_COMMIT,
    UNSAFE_BIP39_SEED_HEX,
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
    for rejection in json.array("rejections") {
        let candidate = hx32(rejection.string("candidate_xonly"));
        let error = accept_identity_key(
            rejection.number("profile_version") as u16,
            rejection.string("network"),
            rejection.string("path"),
            &candidate,
        )
        .expect_err(rejection.string("id"));
        assert_eq!(
            error,
            rejection.string("reason"),
            "{}",
            rejection.string("id")
        );
    }
}

#[test]
fn seal_policy_reject_cases_from_the_vector_checker() {
    use crate::{recovery_policy_valid, seal_bindings_valid, RecoveryPolicy};
    let bad_delay = RecoveryPolicy {
        version: 1,
        sequence: 1,
        threshold: 2,
        key_ids: vec![[1; 32], [2; 32], [3; 32]],
        delay_blocks: 0,
        cancellation_rule: 1,
    };
    assert!(recovery_policy_valid(&bad_delay).is_err());
    let mut wide = bad_delay.clone();
    wide.delay_blocks = 65_536;
    assert!(recovery_policy_valid(&wide).is_err());
    let mut threshold = bad_delay;
    threshold.delay_blocks = 144;
    threshold.threshold = 4;
    assert!(recovery_policy_valid(&threshold).is_err());
    let fixture = git_show("tests/vectors/seal-script-v0.1.json");
    let parsed = parse_json(&fixture);
    let case = parsed.obj("cases").obj("two_controllers_odd");
    let controllers = bindings(&case, "controller_seal_bindings", "controller_key_id");
    let recovery = bindings(&case, "recovery_seal_bindings", "recovery_key_id");
    let controller_ids = controllers
        .iter()
        .map(|binding| binding.authorizing_key_id)
        .collect::<Vec<_>>();
    let recovery_ids = recovery
        .iter()
        .map(|binding| binding.authorizing_key_id)
        .collect::<Vec<_>>();
    assert!(
        seal_bindings_valid(&controllers, &recovery, &controller_ids, &recovery_ids, &[]).is_ok()
    );
    let mut reversed = controllers.clone();
    reversed.reverse();
    assert!(
        seal_bindings_valid(&reversed, &recovery, &controller_ids, &recovery_ids, &[]).is_err()
    );
    let mut short_recovery = recovery.clone();
    short_recovery.pop();
    assert!(seal_bindings_valid(
        &controllers,
        &short_recovery,
        &controller_ids,
        &recovery_ids,
        &[]
    )
    .is_err());
    let stale = [SealBinding {
        authorizing_key_id: [0xff; 32],
        seal_xonly: controllers[0].seal_xonly,
    }];
    assert!(seal_bindings_valid(&stale, &recovery, &controller_ids, &recovery_ids, &[]).is_err());
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
