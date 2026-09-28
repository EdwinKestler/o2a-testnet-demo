//! Bounded O2A-CANON-1 readers.
//!
//! `decode_payload` mirrors `decode_payload` in
//! `tests/vectors/check_protocol_objects.py`. `parse_claim` and
//! `evaluate_name_claim` mirror the same names in
//! `tests/vectors/check_vectors.py`.

use crate::encode::{ControllerEntry, RecoveryPolicy, ResultingState, SealBinding};
use crate::{key_id, signature_accepts};

const MAX_BYTES: usize = 1_048_576;
const MAX_TEXT: usize = 4_096;
const MAX_LIST: u32 = 4_096;

struct Reader<'a> {
    payload: &'a [u8],
    offset: usize,
    truncated: &'static str,
}

impl<'a> Reader<'a> {
    fn new(payload: &'a [u8], truncated: &'static str) -> Self {
        Self {
            payload,
            offset: 0,
            truncated,
        }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], &'static str> {
        let end = self.offset.checked_add(length).ok_or(self.truncated)?;
        if end > self.payload.len() {
            return Err(self.truncated);
        }
        let value = &self.payload[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, &'static str> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, &'static str> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn bytes_field(
        &mut self,
        maximum: usize,
        oversized: &'static str,
    ) -> Result<&'a [u8], &'static str> {
        let length = self.u32()? as usize;
        if length > maximum {
            return Err(oversized);
        }
        self.take(length)
    }

    fn text(&mut self, maximum: usize) -> Result<(), &'static str> {
        let value = self.bytes_field(maximum, "oversized bytes")?;
        if value.contains(&0) {
            return Err("NUL in text");
        }
        std::str::from_utf8(value).map_err(|_| "invalid UTF-8")?;
        Ok(())
    }

    fn option_fixed(&mut self, length: usize) -> Result<(), &'static str> {
        self.option_bytes(length).map(|_| ())
    }

    fn option_bytes(&mut self, length: usize) -> Result<Option<Vec<u8>>, &'static str> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.take(length)?.to_vec())),
            _ => Err("invalid option marker"),
        }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], &'static str> {
        let bytes = self.take(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn list_count(&mut self) -> Result<u32, &'static str> {
        let count = self.u32()?;
        if count > MAX_LIST {
            return Err("oversized list");
        }
        Ok(count)
    }

    fn done(&self, trailing: &'static str) -> Result<(), &'static str> {
        if self.offset != self.payload.len() {
            Err(trailing)
        } else {
            Ok(())
        }
    }
}

/// Consumes one protocol-object payload. Truncation and trailing bytes fail.
pub fn decode_payload(payload: &[u8]) -> Result<(), &'static str> {
    let mut reader = Reader::new(payload, "truncated payload");
    let _version = reader.u16()?;
    let _network = reader.u8()?;
    let object_type = reader.u16()?;
    reader.take(32)?;
    reader.option_fixed(32)?;
    reader.take(32)?;
    reader.u8()?;
    reader.u16()?;
    match object_type {
        1 => {
            reader.u16()?;
            reader.take(32)?;
            decode_state(&mut reader)?;
        }
        2 => {
            reader.u8()?;
            decode_state(&mut reader)?;
        }
        3 => {
            reader.u8()?;
            reader.take(32)?;
            reader.u32()?;
            decode_state(&mut reader)?;
        }
        5 => {
            reader.u8()?;
            reader.take(32)?;
            reader.text(MAX_TEXT)?;
            reader.bytes_field(MAX_BYTES, "oversized bytes")?;
            let count = reader.list_count()?;
            for _ in 0..count {
                reader.take(32)?;
            }
            reader.option_fixed(32)?;
            reader.take(32)?;
        }
        6 => {
            reader.take(32)?;
            reader.text(MAX_TEXT)?;
            let count = reader.list_count()?;
            for _ in 0..count {
                reader.take(32)?;
            }
            reader.option_fixed(32)?;
            reader.take(32)?;
        }
        7 => {
            reader.take(32)?;
            reader.text(MAX_TEXT)?;
            reader.option_fixed(32)?;
            reader.take(32)?;
        }
        8 => {
            reader.u8()?;
            reader.text(MAX_TEXT)?;
            reader.u16()?;
            reader.take(32)?;
            reader.u64()?;
            reader.u64()?;
            reader.take(32)?;
        }
        9 => {
            reader.take(32)?;
            reader.u8()?;
            reader.text(MAX_TEXT)?;
            reader.take(32)?;
            reader.u64()?;
            reader.u64()?;
            reader.u8()?;
            let marker = reader.u8()?;
            if marker == 1 {
                decode_content_reference(&mut reader)?;
            } else if marker != 0 {
                return Err("invalid option marker");
            }
        }
        10 => {
            reader.u8()?;
            reader.u8()?;
            reader.bytes_field(MAX_BYTES, "oversized bytes")?;
            reader.u16()?;
            reader.u64()?;
            reader.option_fixed(8)?;
            reader.option_fixed(32)?;
            reader.take(32)?;
        }
        11 => {
            reader.u8()?;
            reader.u32()?;
            reader.take(32)?;
            let temporal = reader.list_count()?;
            for _ in 0..temporal {
                reader.take(32)?;
            }
            let places = reader.list_count()?;
            for _ in 0..places {
                reader.take(32)?;
            }
            let participants = reader.list_count()?;
            for _ in 0..participants {
                reader.take(32)?;
                reader.text(MAX_TEXT)?;
            }
            reader.option_fixed(32)?;
            let contents = reader.list_count()?;
            for _ in 0..contents {
                decode_content_reference(&mut reader)?;
            }
            reader.take(32)?;
            reader.option_fixed(32)?;
        }
        _ => return Err("unknown object type"),
    }
    reader.done("trailing payload bytes")
}

fn decode_content_reference(reader: &mut Reader<'_>) -> Result<(), &'static str> {
    reader.text(127)?;
    reader.u64()?;
    reader.take(32)?;
    Ok(())
}

/// Returns the resulting state from a genesis, transition, or recovery payload.
///
/// Truncation and trailing bytes fail, as they do in [`decode_payload`].
pub fn decode_identity_state(payload: &[u8]) -> Result<ResultingState, &'static str> {
    let mut reader = Reader::new(payload, "truncated payload");
    let _version = reader.u16()?;
    let _network = reader.u8()?;
    let object_type = reader.u16()?;
    reader.take(32)?;
    reader.option_fixed(32)?;
    reader.take(32)?;
    reader.u8()?;
    reader.u16()?;
    match object_type {
        1 => {
            reader.u16()?;
            reader.take(32)?;
        }
        2 => {
            reader.u8()?;
        }
        3 => {
            reader.u8()?;
            reader.take(32)?;
            reader.u32()?;
        }
        _ => return Err("object does not establish an identity state"),
    }
    let state = parse_resulting_state(&mut reader)?;
    reader.done("trailing payload bytes")?;
    Ok(state)
}

fn parse_resulting_state(reader: &mut Reader<'_>) -> Result<ResultingState, &'static str> {
    let sequence = reader.u64()?;
    let previous_state = optional_array(reader, 32)?;
    let previous_seal = optional_array(reader, 36)?;
    let next_seal = reader.array()?;
    let controller_count = reader.list_count()?;
    let mut controllers = Vec::with_capacity(controller_count as usize);
    for _ in 0..controller_count {
        reader.take(32)?;
        let xonly = reader.array()?;
        reader.u8()?;
        let capability_count = reader.list_count()?;
        let mut capabilities = Vec::with_capacity(capability_count as usize);
        for _ in 0..capability_count {
            capabilities.push(reader.u16()?);
        }
        controllers.push(ControllerEntry {
            xonly,
            capabilities,
        });
    }
    let version = reader.u16()?;
    let recovery_sequence = reader.u64()?;
    let threshold = reader.u16()?;
    let recovery_key_count = reader.list_count()?;
    let mut key_ids = Vec::with_capacity(recovery_key_count as usize);
    for _ in 0..recovery_key_count {
        key_ids.push(reader.array()?);
    }
    let delay_blocks = reader.u32()?;
    let cancellation_rule = reader.u8()?;
    reader.u16()?;
    let controller_binding_count = reader.list_count()?;
    let mut controller_bindings = Vec::with_capacity(controller_binding_count as usize);
    for _ in 0..controller_binding_count {
        controller_bindings.push(binding_pair(reader)?);
    }
    let recovery_binding_count = reader.list_count()?;
    let mut recovery_bindings = Vec::with_capacity(recovery_binding_count as usize);
    for _ in 0..recovery_binding_count {
        recovery_bindings.push(binding_pair(reader)?);
    }
    reader.option_fixed(32)?;
    let lifecycle_status = reader.u8()?;
    reader.option_fixed(32)?;
    reader.option_fixed(32)?;
    Ok(ResultingState {
        sequence,
        previous_state,
        previous_seal,
        next_seal,
        controllers,
        recovery: RecoveryPolicy {
            version,
            sequence: recovery_sequence,
            threshold,
            key_ids,
            delay_blocks,
            cancellation_rule,
        },
        controller_bindings,
        recovery_bindings,
        lifecycle_status,
    })
}

fn optional_array<const N: usize>(
    reader: &mut Reader<'_>,
    length: usize,
) -> Result<Option<[u8; N]>, &'static str> {
    let Some(bytes) = reader.option_bytes(length)? else {
        return Ok(None);
    };
    let mut out = [0u8; N];
    if bytes.len() != N {
        return Err("truncated payload");
    }
    out.copy_from_slice(&bytes);
    Ok(Some(out))
}

fn binding_pair(reader: &mut Reader<'_>) -> Result<SealBinding, &'static str> {
    Ok(SealBinding {
        authorizing_key_id: reader.array()?,
        seal_xonly: reader.array()?,
    })
}

fn decode_state(reader: &mut Reader<'_>) -> Result<(), &'static str> {
    reader.u64()?;
    reader.option_fixed(32)?;
    reader.option_fixed(36)?;
    reader.take(36)?;
    let controllers = reader.list_count()?;
    for _ in 0..controllers {
        reader.take(32)?;
        reader.take(32)?;
        reader.u8()?;
        let capabilities = reader.list_count()?;
        for _ in 0..capabilities {
            reader.u16()?;
        }
    }
    reader.u16()?;
    reader.u64()?;
    reader.u16()?;
    let recovery_keys = reader.list_count()?;
    for _ in 0..recovery_keys {
        reader.take(32)?;
    }
    reader.u32()?;
    reader.u8()?;
    reader.u16()?;
    let controller_bindings = reader.list_count()?;
    for _ in 0..controller_bindings {
        reader.take(64)?;
    }
    let recovery_bindings = reader.list_count()?;
    for _ in 0..recovery_bindings {
        reader.take(64)?;
    }
    reader.option_fixed(32)?;
    reader.u8()?;
    reader.option_fixed(32)?;
    reader.option_fixed(32)?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ParsedClaim {
    pub network: u8,
    pub version: u16,
    pub object_type: u16,
    pub signing_entity: [u8; 32],
    pub authorizing_state: Option<[u8; 32]>,
    pub signing_key_id: [u8; 32],
    pub key_role: u8,
    pub capability: u16,
    pub subject: [u8; 32],
}

/// Mirrors `parse_claim` in `check_vectors.py`.
pub fn parse_claim(payload: &[u8]) -> Result<ParsedClaim, &'static str> {
    let mut reader = Reader::new(payload, "truncated claim");
    let version = reader.u16()?;
    let network = reader.u8()?;
    let object_type = reader.u16()?;
    let signing_entity = fixed32(&mut reader)?;
    let authorizing_state = match reader.u8()? {
        0 => None,
        1 => Some(fixed32(&mut reader)?),
        _ => return Err("invalid option marker"),
    };
    let signing_key_id = fixed32(&mut reader)?;
    let key_role = reader.u8()?;
    let capability = reader.u16()?;
    let subject = fixed32(&mut reader)?;
    claim_text(&mut reader)?;
    claim_bytes(&mut reader)?;
    if reader.u8()? != 0 {
        return Err("fixture requires absent claim context");
    }
    reader.take(32)?;
    claim_option32(&mut reader)?;
    claim_option32(&mut reader)?;
    reader.done("trailing claim bytes")?;
    Ok(ParsedClaim {
        network,
        version,
        object_type,
        signing_entity,
        authorizing_state,
        signing_key_id,
        key_role,
        capability,
        subject,
    })
}

fn fixed32(reader: &mut Reader<'_>) -> Result<[u8; 32], &'static str> {
    Ok(reader.take(32)?.try_into().unwrap())
}

fn claim_text(reader: &mut Reader<'_>) -> Result<(), &'static str> {
    let length = reader.u32()? as usize;
    if length > MAX_TEXT {
        return Err("oversized text field");
    }
    let value = reader.take(length)?;
    if value.contains(&0) {
        return Err("NUL in canonical text");
    }
    std::str::from_utf8(value).map_err(|_| "invalid UTF-8")?;
    Ok(())
}

fn claim_bytes(reader: &mut Reader<'_>) -> Result<(), &'static str> {
    let length = reader.u32()? as usize;
    if length > MAX_BYTES {
        return Err("oversized bytes field");
    }
    reader.take(length)?;
    Ok(())
}

fn claim_option32(reader: &mut Reader<'_>) -> Result<(), &'static str> {
    match reader.u8()? {
        0 => Ok(()),
        1 => {
            reader.take(32)?;
            Ok(())
        }
        _ => Err("invalid option marker"),
    }
}

#[derive(Clone, Debug)]
pub struct ClaimAuthorization<'a> {
    pub entity: &'a [u8; 32],
    pub state: Option<&'a [u8; 32]>,
    pub key_id: &'a [u8; 32],
    pub public_key: &'a [u8; 32],
    pub key_role: u8,
    pub capabilities: &'a [u16],
}

/// Mirrors `evaluate_name_claim` in `check_vectors.py`.
pub fn evaluate_name_claim(
    payload: &[u8],
    signature: [u8; 64],
    public_key: [u8; 32],
    expected_network: u8,
    authorization: &ClaimAuthorization<'_>,
) -> Result<(), &'static str> {
    let claim = parse_claim(payload)?;
    if !signature_accepts("O2A/v0.1/claim", payload, signature, public_key) {
        return Err("invalid signature");
    }
    if !matches!(claim.network, 0..=4) || !matches!(expected_network, 0..=4) {
        return Err("unknown Bitcoin network");
    }
    if claim.network != expected_network {
        return Err("wrong verifier network");
    }
    if claim.version != 1 || claim.object_type != 4 || claim.capability != 4 {
        return Err("object/domain/capability mismatch");
    }
    if claim.key_role != 1 {
        return Err("wrong signing-key role");
    }
    let Some(authorizing_state) = claim.authorizing_state else {
        return Err("missing authorizing state");
    };
    if claim.signing_key_id != key_id(claim.key_role, public_key) {
        return Err("signing key ID does not match role and public key");
    }
    if claim.signing_entity != claim.subject {
        return Err("name claim is not self-issued");
    }
    if authorization.entity != &claim.signing_entity
        || authorization.state != Some(&authorizing_state)
        || authorization.key_id != &claim.signing_key_id
        || authorization.public_key != &public_key
        || authorization.key_role != claim.key_role
        || !authorization.capabilities.contains(&claim.capability)
    {
        return Err("signer is not authorized by the stated fixture state");
    }
    Ok(())
}
