use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use bitcoin_hashes::{sha256, Hash};
use o2a_demo_core::{
    evaluate_lineage, format_lineage_report, CurrentSealView, InclusionProof, LineageEvidence,
    SealFact,
};

pub struct Electrum {
    stream: TcpStream,
    next_id: u64,
}

impl Electrum {
    pub fn connect(url: &str) -> Result<Self> {
        let host = url
            .trim_start_matches("tcp://")
            .trim_start_matches("ssl://");
        let stream = TcpStream::connect(host).with_context(|| format!("electrum {host}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        Ok(Self { stream, next_id: 1 })
    }

    pub fn call(&mut self, method: &str, params: &str) -> Result<String> {
        let id = self.next_id;
        self.next_id += 1;
        let request = format!(r#"{{"id":{id},"method":"{method}","params":{params}}}"#);
        self.stream.write_all(request.as_bytes())?;
        self.stream.write_all(b"\n")?;
        self.stream.flush()?;
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            let read = self.stream.read(&mut byte)?;
            if read == 0 {
                break;
            }
            if byte[0] == b'\n' {
                break;
            }
            buf.push(byte[0]);
        }
        let text = String::from_utf8(buf)?;
        if text.contains("\"error\":")
            && !text.contains("\"error\": null")
            && !text.contains("\"error\":null")
        {
            bail!("electrum {method} failed: {text}");
        }
        Ok(text)
    }
}

fn json_string_after(text: &str, key: &str) -> Result<String> {
    let pattern = format!("\"{key}\"");
    let start = text
        .find(&pattern)
        .with_context(|| format!("missing {key}"))?;
    let rest = &text[start + pattern.len()..];
    let quote = rest.find('"').context("string")? + 1;
    let end = rest[quote..].find('"').context("string end")? + quote;
    Ok(rest[quote..end].to_owned())
}

fn json_number_after(text: &str, key: &str) -> Result<u32> {
    let value = json_i64_after(text, key)?;
    u32::try_from(value).context("number")
}

fn json_i64_after(text: &str, key: &str) -> Result<i64> {
    let pattern = format!("\"{key}\"");
    let start = text
        .find(&pattern)
        .with_context(|| format!("missing {key}"))?;
    let rest = text[start + pattern.len()..]
        .trim_start_matches(|ch: char| ch == ':' || ch.is_whitespace());
    let negative = rest.starts_with('-');
    let digits: String = rest[usize::from(negative)..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        bail!("missing {key}");
    }
    let value: i64 = digits.parse()?;
    Ok(if negative { -value } else { value })
}

struct HistoryItem {
    txid: String,
    height: i64,
}

fn json_objects(text: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else {
            break;
        };
        objects.push(rest[start..start + end + 1].to_owned());
        rest = &rest[start + end + 1..];
    }
    objects
}

fn history_items(history: &str) -> Result<Vec<HistoryItem>> {
    let mut items = Vec::new();
    for object in json_objects(history) {
        if !object.contains("\"tx_hash\"") {
            continue;
        }
        items.push(HistoryItem {
            txid: json_string_after(&object, "tx_hash")?,
            height: json_i64_after(&object, "height")?,
        });
    }
    Ok(items)
}

fn hex_internal(value: &str) -> Result<[u8; 32]> {
    let mut bytes = hex::decode(value).context("hex")?;
    if bytes.len() != 32 {
        bail!("expected 32 bytes");
    }
    bytes.reverse();
    Ok(bytes.try_into().unwrap())
}

fn header_at(client: &mut Electrum, height: u32) -> Result<[u8; 80]> {
    let text = client.call("blockchain.block.header", &format!("[{height}]"))?;
    let hex_header = json_string_after(&text, "result")?;
    let bytes = hex::decode(hex_header)?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("header is not 80 bytes"))
}

pub struct ObservedSeal {
    pub best_height: u32,
    pub best_hash: [u8; 32],
    pub creation: InclusionProof,
    pub observed_script: Vec<u8>,
    pub unspent: bool,
    pub spend: Option<InclusionProof>,
}

pub struct TrackedSeal {
    pub expected_script: Vec<u8>,
    pub outpoint: String,
    pub funding_txid: String,
}

fn scripthash(script: &[u8]) -> String {
    let mut hash = sha256::Hash::hash(script).to_byte_array();
    hash.reverse();
    hex::encode(hash)
}

fn fetch_raw(client: &mut Electrum, txid: &str) -> Result<Vec<u8>> {
    let raw = client.call("blockchain.transaction.get", &format!("[\"{txid}\"]"))?;
    let raw_hex = json_string_after(&raw, "result")?;
    Ok(hex::decode(raw_hex)?)
}

fn history_for(client: &mut Electrum, script: &[u8]) -> Result<String> {
    client.call(
        "blockchain.scripthash.get_history",
        &format!("[\"{}\"]", scripthash(script)),
    )
}

fn unspent_outpoint(text: &str, txid: &str, vout: u32) -> Result<bool> {
    for object in json_objects(text) {
        if !object.contains("\"tx_hash\"") || !object.contains("\"tx_pos\"") {
            continue;
        }
        if json_string_after(&object, "tx_hash")? == txid
            && json_number_after(&object, "tx_pos")? == vout
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn absent_proof(client: &mut Electrum, txid: &str, best_height: u32) -> Result<InclusionProof> {
    let header = if best_height == 0 {
        [0u8; 80]
    } else {
        header_at(client, best_height)?
    };
    Ok(InclusionProof {
        txid: hex_internal(txid).unwrap_or([0u8; 32]),
        index: 0,
        siblings: vec![[0x11; 32]],
        header,
        height: best_height,
    })
}

pub fn observe_outpoint(
    client: &mut Electrum,
    outpoint: &str,
    funding_txid_display: &str,
) -> Result<ObservedSeal> {
    let tip = client.call("blockchain.headers.subscribe", "[]")?;
    let best_height = json_number_after(&tip, "height")?;
    let tip_header = hex::decode(json_string_after(&tip, "hex")?)?;
    let mut best_hash = bitcoin_hashes::sha256d::Hash::hash(&tip_header).to_byte_array();
    best_hash.reverse();
    let (_txid_display, vout) = outpoint.split_once(':').context("outpoint")?;
    let vout: u32 = vout.parse()?;
    let raw = fetch_raw(client, funding_txid_display)?;
    let observed_script = output_script(&raw, vout)?;
    let history = history_for(client, &observed_script)?;
    let items = history_items(&history)?;
    let creation_height = items
        .iter()
        .find(|item| item.txid == funding_txid_display && item.height > 0)
        .map(|item| item.height as u32);
    let creation = match creation_height {
        Some(height) => inclusion_proof(client, funding_txid_display, height)?,
        None => absent_proof(client, funding_txid_display, best_height)?,
    };
    let unspent_text = client.call(
        "blockchain.scripthash.listunspent",
        &format!("[\"{}\"]", scripthash(&observed_script)),
    )?;
    let unspent = unspent_outpoint(&unspent_text, funding_txid_display, vout)?;
    let spend = if unspent {
        None
    } else if let Some(spender) = confirmed_spender(client, &items, funding_txid_display, vout)? {
        Some(inclusion_proof(client, &spender.0, spender.1)?)
    } else {
        None
    };
    Ok(ObservedSeal {
        best_height,
        best_hash,
        creation,
        observed_script,
        unspent,
        spend,
    })
}

fn confirmed_spender(
    client: &mut Electrum,
    items: &[HistoryItem],
    funding: &str,
    vout: u32,
) -> Result<Option<(String, u32)>> {
    for item in items {
        if item.txid == funding || item.height <= 0 {
            continue;
        }
        let raw = fetch_raw(client, &item.txid)?;
        if spends_outpoint(&raw, funding, vout)? {
            return Ok(Some((item.txid.clone(), item.height as u32)));
        }
    }
    Ok(None)
}

pub fn inclusion_proof(client: &mut Electrum, txid: &str, height: u32) -> Result<InclusionProof> {
    let text = client.call(
        "blockchain.transaction.get_merkle",
        &format!("[\"{txid}\", {height}]"),
    )?;
    let index = json_number_after(&text, "pos")?;
    let header = header_at(client, height)?;
    let siblings = merkle_siblings(&text)?;
    Ok(InclusionProof {
        txid: hex_internal(txid)?,
        index,
        siblings,
        header,
        height,
    })
}

pub fn locate_inclusion(
    client: &mut Electrum,
    txid: &str,
    scripts: &[Vec<u8>],
    best_height: u32,
) -> Result<InclusionProof> {
    for script in scripts {
        let items = history_items(&history_for(client, script)?)?;
        if let Some(item) = items
            .iter()
            .find(|item| item.txid == txid && item.height > 0)
        {
            return match inclusion_proof(client, txid, item.height as u32) {
                Ok(proof) => Ok(proof),
                Err(error) => {
                    let _ = error;
                    absent_proof(client, txid, best_height)
                }
            };
        }
    }
    absent_proof(client, txid, best_height)
}

fn merkle_siblings(text: &str) -> Result<Vec<[u8; 32]>> {
    let Some(start) = text.find("\"merkle\"") else {
        bail!("missing merkle");
    };
    let list = &text[start..];
    let open = list.find('[').context("merkle list")?;
    let close = list[open..].find(']').context("merkle end")? + open;
    let body = &list[open + 1..close];
    let mut siblings = Vec::new();
    for part in body.split(',') {
        let trimmed = part.trim().trim_matches('"');
        if trimmed.is_empty() {
            continue;
        }
        siblings.push(hex_internal(trimmed)?);
    }
    Ok(siblings)
}

fn output_script(raw: &[u8], vout: u32) -> Result<Vec<u8>> {
    let mut cursor = 4usize;
    if raw.get(cursor) == Some(&0) && raw.get(cursor + 1) == Some(&1) {
        cursor += 2;
    }
    let (input_count, next) = compact(raw, cursor)?;
    cursor = next;
    for _ in 0..input_count {
        cursor += 36;
        let (script_len, next) = compact(raw, cursor)?;
        cursor = next + script_len as usize;
        cursor += 4;
    }
    let (output_count, next) = compact(raw, cursor)?;
    cursor = next;
    if u64::from(vout) >= output_count {
        bail!("vout {vout} is outside the transaction");
    }
    for index in 0..output_count {
        cursor += 8;
        let (script_len, next) = compact(raw, cursor)?;
        cursor = next;
        if index == u64::from(vout) {
            return Ok(raw[cursor..cursor + script_len as usize].to_vec());
        }
        cursor += script_len as usize;
    }
    bail!("output script was not read")
}

fn spends_outpoint(raw: &[u8], funding_display: &str, vout: u32) -> Result<bool> {
    let mut want = hex::decode(funding_display).context("funding txid")?;
    if want.len() != 32 {
        bail!("funding txid is not 32 bytes");
    }
    want.reverse();
    let mut cursor = 4usize;
    if raw.get(cursor) == Some(&0) && raw.get(cursor + 1) == Some(&1) {
        cursor += 2;
    }
    let (input_count, next) = compact(raw, cursor)?;
    cursor = next;
    for _ in 0..input_count {
        if raw.len() < cursor + 36 {
            bail!("transaction input is truncated");
        }
        let prev = &raw[cursor..cursor + 32];
        let prev_vout = u32::from_le_bytes(raw[cursor + 32..cursor + 36].try_into()?);
        if prev == want.as_slice() && prev_vout == vout {
            return Ok(true);
        }
        cursor += 36;
        let (script_len, next) = compact(raw, cursor)?;
        cursor = next + script_len as usize + 4;
    }
    Ok(false)
}

fn compact(raw: &[u8], cursor: usize) -> Result<(u64, usize)> {
    let marker = *raw.get(cursor).context("compact size")?;
    if marker < 253 {
        Ok((u64::from(marker), cursor + 1))
    } else if marker == 253 {
        let value = u16::from_le_bytes(raw[cursor + 1..cursor + 3].try_into()?);
        Ok((u64::from(value), cursor + 3))
    } else {
        bail!("compact size is too wide for this parser")
    }
}

pub fn gather_report(
    client: &mut Electrum,
    seals: &[TrackedSeal],
    current: usize,
    anchor_txid: &str,
    o2a_objects_ok: bool,
    rgb: &'static str,
    required_depth: u32,
    skip_observation: bool,
) -> Result<String> {
    if seals.is_empty() || current >= seals.len() {
        bail!("lineage has no current seal");
    }
    let mut observed = Vec::new();
    for seal in seals {
        observed.push(observe_outpoint(
            client,
            &seal.outpoint,
            &seal.funding_txid,
        )?);
    }
    let tip = &observed[0];
    let anchor = locate_inclusion(
        client,
        anchor_txid,
        &observed
            .iter()
            .map(|item| item.observed_script.clone())
            .collect::<Vec<_>>(),
        tip.best_height,
    )?;
    let current_view = &observed[current];
    let anchor_internal = hex_internal(anchor_txid)?;
    let valid_transition = current_view
        .spend
        .as_ref()
        .is_some_and(|spend| spend.txid == anchor_internal);
    let evidence = LineageEvidence {
        seals: seals
            .iter()
            .zip(observed.iter())
            .map(|(seal, item)| SealFact {
                expected_script: seal.expected_script.clone(),
                observed_script: item.observed_script.clone(),
                creation: item.creation.clone(),
            })
            .collect(),
        anchor: Some(anchor.clone()),
        observation: if skip_observation {
            None
        } else {
            Some(CurrentSealView {
                unspent: current_view.unspent,
                spend: current_view.spend.clone(),
            })
        },
        o2a_ok: o2a_objects_ok,
        valid_transition,
        best_height: tip.best_height,
        required_depth,
    };
    let report = evaluate_lineage(&evidence, rgb);
    Ok(format!(
        "best_height={}\nbest_block_hash={}\nsource_url=tcp://electrs:50001\nrequired_depth={required_depth}\nanchor_txid={anchor_txid}\nanchor_height={}\ncurrent_outpoint={}\n{}",
        tip.best_height,
        hex::encode(tip.best_hash),
        anchor.height,
        seals[current].outpoint,
        format_lineage_report(&report)
    ))
}
