use o2a_demo_core::{
    demo_genesis_state, demo_keys, demo_rotation_state, encode_seal_policy, seal_for_state,
    ResultingState, NUMS_X,
};

const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut chk = 1u32;
    for value in values {
        let top = chk >> 25;
        chk = ((chk & 0x01ff_ffff) << 5) ^ u32::from(*value);
        for (bit, gen) in GEN.iter().enumerate() {
            if (top >> bit) & 1 == 1 {
                chk ^= gen;
            }
        }
    }
    chk
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for byte in hrp.bytes() {
        out.push(byte >> 5);
    }
    out.push(0);
    for byte in hrp.bytes() {
        out.push(byte & 31);
    }
    out
}

fn convert_bits(data: &[u8]) -> Vec<u8> {
    let mut acc = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for value in data {
        acc = (acc << 8) | u32::from(*value);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 31) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 31) as u8);
    }
    out
}

pub fn regtest_address(output_key: &[u8; 32]) -> String {
    let hrp = "bcrt";
    let mut data = vec![1u8];
    data.extend(convert_bits(output_key));
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(&data);
    values.extend([0, 0, 0, 0, 0, 0]);
    let checksum = polymod(&values) ^ 0x2bc8_30a3;
    for shift in (0..6).rev() {
        data.push(((checksum >> (5 * shift)) & 31) as u8);
    }
    let body: String = data
        .iter()
        .map(|value| CHARSET[*value as usize] as char)
        .collect();
    format!("{hrp}1{body}")
}

pub fn core_descriptor(state: &ResultingState) -> String {
    let mut recovery = state
        .recovery_bindings
        .iter()
        .map(|binding| binding.seal_xonly)
        .collect::<Vec<_>>();
    recovery.sort();
    let keys = recovery
        .iter()
        .map(hex::encode)
        .collect::<Vec<_>>()
        .join(",");
    let older = format!(
        "and_v(v:multi_a({},{}),older({}))",
        state.recovery.threshold, keys, state.recovery.delay_blocks
    );
    let mut controllers = state
        .controller_bindings
        .iter()
        .map(|binding| binding.seal_xonly)
        .collect::<Vec<_>>();
    controllers.sort();
    let controller = controllers.first().map(hex::encode).unwrap_or_default();
    format!("tr({},{{pk({controller}),{older}}})", hex::encode(NUMS_X))
}

pub struct PrepareText {
    pub text: String,
    pub script_pubkey: Vec<u8>,
    pub policy: Vec<u8>,
    pub paths: String,
}

pub fn prepare_state(stage: &str) -> Result<PrepareText, &'static str> {
    let state = match stage {
        "genesis" => demo_genesis_state([0; 36]),
        "rotate" | "recover" => demo_rotation_state(1, [0; 32], [0; 36], [0; 36]),
        _ => return Err("stage must be genesis, rotate, or recover"),
    };
    let script = seal_for_state(&state)?;
    let keys = demo_keys();
    let entity = o2a_demo_core::demo_entity_index();
    let paths = if stage == "genesis" {
        format!(
            "m/1'/{entity}'/1'/0' m/1'/{entity}'/4'/0' m/1'/{entity}'/2'/0' m/1'/{entity}'/4'/2' m/1'/{entity}'/2'/1' m/1'/{entity}'/4'/3' m/1'/{entity}'/2'/2' m/1'/{entity}'/4'/4'"
        )
    } else {
        format!("m/1'/{entity}'/1'/1' m/1'/{entity}'/4'/1'")
    };
    let _ = keys;
    let policy = encode_seal_policy(&state.controller_bindings, &state.recovery_bindings);
    let address = regtest_address(&script.output_key);
    let text = format!(
        "stage={stage}\naddress={address}\nscript_pubkey={}\npolicy={}\npaths={paths}\ndescriptor={}\n",
        hex::encode(&script.script_pubkey),
        hex::encode(&policy),
        core_descriptor(&state)
    );
    Ok(PrepareText {
        text,
        script_pubkey: script.script_pubkey,
        policy,
        paths,
    })
}
