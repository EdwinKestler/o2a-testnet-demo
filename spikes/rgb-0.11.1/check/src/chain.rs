//! Regtest RPC plus an electrs-backed witness resolver. No RGB wallet.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use rgbstd::validation::{ResolveWitness, WitnessResolverError, WitnessStatus};
use rgbstd::vm::{WitnessOrd, WitnessPos};
use rgbstd::{ChainNet, Txid};
use serde_json::{json, Value};

use rgbstd::bitcoin::consensus::encode::deserialize;
use rgbstd::bitcoin::Transaction;

pub struct Node {
    host: String,
    port: u16,
    auth: String,
    pub miner: String,
    refuse_broadcast: bool,
}

impl Node {
    pub fn connect() -> Result<Self, String> {
        Self::connect_wallet(true)
    }

    /// `create_miner` is for the disposable regtest wallet. Signet reads leave it false.
    pub fn connect_wallet(create_miner: bool) -> Result<Self, String> {
        Self::connect_profile(&crate::profile::load()?, create_miner)
    }

    pub fn connect_profile(
        active: &crate::profile::NetworkProfile,
        create_miner: bool,
    ) -> Result<Self, String> {
        crate::profile::allow_transport(active)?;
        let url = active
            .rpc_url
            .clone()
            .ok_or_else(|| "backend endpoints are not configured".to_string())?;
        let (host, port) = parse_http(&url)?;
        let auth = if let (Ok(user), Ok(pass)) = (
            std::env::var("BITCOIN_RPC_USER"),
            std::env::var("BITCOIN_RPC_PASSWORD"),
        ) {
            base64(format!("{user}:{pass}").as_bytes())
        } else {
            let cookie_path = active
                .cookie_path
                .clone()
                .ok_or_else(|| "cookie path is not configured".to_string())?;
            let cookie = std::fs::read_to_string(&cookie_path)
                .map_err(|err| format!("cookie {cookie_path}: {err}"))?;
            base64(cookie.trim().as_bytes())
        };
        let node = Self {
            host,
            port,
            auth,
            miner: active.wallet.clone(),
            refuse_broadcast: active.kind == crate::profile::NetworkKind::Mainnet,
        };
        if create_miner && active.create_miner_wallet {
            match node.call(false, "createwallet", json!([node.miner])) {
                Ok(_) => {}
                Err(err)
                    if err.contains("already exists")
                        || err.contains("Database already exists") => {}
                Err(err) => return Err(err),
            }
        }
        Ok(node)
    }

    pub fn call(&self, wallet: bool, method: &str, params: Value) -> Result<Value, String> {
        // The allowlist is decided here, before raw() opens a socket.
        crate::profile::mainnet_method_gate(self.refuse_broadcast, method)?;
        let response = self.raw(wallet, method, params)?;
        if let Some(err) = response.get("error").filter(|value| !value.is_null()) {
            return Err(err.to_string());
        }
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    pub fn raw(&self, wallet: bool, method: &str, params: Value) -> Result<Value, String> {
        let path = if wallet {
            format!("/wallet/{}", self.miner)
        } else {
            "/".into()
        };
        let body = json!({"jsonrpc": "1.0", "id": method, "method": method, "params": params});
        let payload = body.to_string();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nAuthorization: Basic {auth}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{payload}",
            host = self.host,
            port = self.port,
            auth = self.auth,
            len = payload.len(),
        );
        let mut stream = TcpStream::connect((self.host.as_str(), self.port))
            .map_err(|err| format!("rpc connect: {err}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|err| err.to_string())?;
        stream
            .write_all(request.as_bytes())
            .map_err(|err| format!("rpc write: {err}"))?;
        let mut buffer = Vec::new();
        stream
            .read_to_end(&mut buffer)
            .map_err(|err| format!("rpc read: {err}"))?;
        let text = String::from_utf8_lossy(&buffer);
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or(text.as_ref());
        let json_text = if body.starts_with(|ch: char| ch.is_ascii_digit()) && body.contains("\r\n")
        {
            decode_chunked(body)
        } else {
            body.trim().to_string()
        };
        serde_json::from_str(&json_text).map_err(|err| format!("rpc json: {err}; body={json_text}"))
    }

    pub fn height(&self) -> Result<u32, String> {
        let value = self.call(false, "getblockcount", json!([]))?;
        value
            .as_u64()
            .map(|height| height as u32)
            .ok_or_else(|| format!("height {value}"))
    }

    pub fn mine(&self, blocks: u64) -> Result<u32, String> {
        let address = self.call(true, "getnewaddress", json!([]))?;
        let address = address
            .as_str()
            .ok_or_else(|| format!("address {address}"))?
            .to_string();
        self.call(true, "generatetoaddress", json!([blocks, address]))?;
        let height = self.height()?;
        wait_electrs(height)?;
        Ok(height)
    }

    pub fn tx_hex(&self, txid: &str) -> Result<(String, Option<u32>, Option<i64>), String> {
        let tx = self.call(false, "getrawtransaction", json!([txid, true]))?;
        let hex = tx
            .get("hex")
            .and_then(Value::as_str)
            .ok_or("missing tx hex")?
            .to_string();
        let height = tx
            .get("blockheight")
            .and_then(Value::as_u64)
            .map(|v| v as u32);
        let time = tx
            .get("blocktime")
            .and_then(Value::as_i64)
            .or_else(|| tx.get("time").and_then(Value::as_i64));
        Ok((hex, height, time))
    }
}

pub struct ElectrumResolver {
    addr: String,
    rpc: Node,
    next_id: u64,
}

impl ElectrumResolver {
    pub fn open() -> Result<Self, String> {
        let active = crate::profile::load()?;
        crate::profile::allow_transport(&active)?;
        let addr = active
            .electrum
            .clone()
            .ok_or_else(|| "backend endpoints are not configured".to_string())?;
        let rpc = if active.kind == crate::profile::NetworkKind::Signet {
            Node::connect_wallet(false)?
        } else {
            Node::connect()?
        };
        Ok(Self {
            addr,
            rpc,
            next_id: 1,
        })
    }

    pub(crate) fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        crate::profile::mainnet_method_gate(self.rpc.refuse_broadcast, method)?;
        let id = self.next_id;
        self.next_id += 1;
        let mut stream =
            TcpStream::connect(&self.addr).map_err(|err| format!("electrum: {err}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .map_err(|err| err.to_string())?;
        let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        stream
            .write_all(format!("{line}\n").as_bytes())
            .map_err(|err| err.to_string())?;
        let mut buffer = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            let read = stream.read(&mut byte).map_err(|err| err.to_string())?;
            if read == 0 {
                break;
            }
            if byte[0] == b'\n' {
                break;
            }
            buffer.push(byte[0]);
            if buffer.len() > 8_000_000 {
                return Err("electrum response is too large".into());
            }
        }
        let text = String::from_utf8(buffer).map_err(|err| err.to_string())?;
        let value: Value =
            serde_json::from_str(&text).map_err(|err| format!("electrum json: {err}"))?;
        if let Some(err) = value.get("error").filter(|item| !item.is_null()) {
            return Err(err.to_string());
        }
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }

    pub fn tip(&mut self) -> Result<u32, String> {
        let result = self.call("blockchain.headers.subscribe", json!([]))?;
        result
            .get("height")
            .and_then(Value::as_u64)
            .map(|height| height as u32)
            .ok_or_else(|| format!("electrum tip {result}"))
    }
}

pub fn chain_net() -> Result<ChainNet, String> {
    let active = crate::profile::load()?;
    if active.rgb_conflict {
        return Err("RGB_CHAIN does not match the selected network".into());
    }
    Ok(active.rgb_chain)
}

pub fn wait_electrs(height: u32) -> Result<(), String> {
    for _ in 0..180 {
        match ElectrumResolver::open() {
            Ok(mut resolver) => match resolver.tip() {
                Ok(tip) if tip >= height => return Ok(()),
                Ok(_) => {}
                Err(_) => {}
            },
            Err(err) if err.contains("cookie") || err.starts_with("rpc") => return Err(err),
            Err(_) => {}
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Err(format!("electrs stayed behind height {height}"))
}

impl ResolveWitness for ElectrumResolver {
    fn resolve_witness(&self, witness_id: Txid) -> Result<WitnessStatus, WitnessResolverError> {
        let mut probe = ElectrumResolver {
            addr: self.addr.clone(),
            rpc: Node {
                host: self.rpc.host.clone(),
                port: self.rpc.port,
                auth: self.rpc.auth.clone(),
                miner: self.rpc.miner.clone(),
                refuse_broadcast: self.rpc.refuse_broadcast,
            },
            next_id: 1,
        };
        let looked = probe.call(
            "blockchain.transaction.get",
            json!([witness_id.to_string(), true]),
        );
        let tx_json = match looked {
            Ok(value) => value,
            Err(err) if err.to_lowercase().contains("no such") || err.contains("not found") => {
                return Ok(WitnessStatus::Unresolved);
            }
            Err(err) => {
                return Err(WitnessResolverError::ResolverIssue(Some(witness_id), err));
            }
        };
        let hex = tx_json
            .get("hex")
            .and_then(Value::as_str)
            .ok_or_else(|| WitnessResolverError::InvalidResolverData)?;
        let raw = hex::decode(hex).map_err(|_| WitnessResolverError::InvalidResolverData)?;
        let tx: Transaction =
            deserialize(&raw).map_err(|_| WitnessResolverError::InvalidResolverData)?;
        let confirmations = tx_json
            .get("confirmations")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if confirmations == 0 {
            return Ok(WitnessStatus::Resolved(tx, WitnessOrd::Tentative));
        }
        let timestamp = tx_json
            .get("blocktime")
            .or_else(|| tx_json.get("time"))
            .and_then(Value::as_i64);
        let height = tx_json
            .get("blockheight")
            .or_else(|| tx_json.get("block_height"))
            .or_else(|| tx_json.get("height"))
            .and_then(Value::as_u64)
            .map(|value| value as u32);
        let (height, timestamp) = match (height, timestamp) {
            (Some(height), Some(timestamp)) => (height, timestamp),
            _ => {
                let blockhash = tx_json
                    .get("blockhash")
                    .and_then(Value::as_str)
                    .ok_or(WitnessResolverError::InvalidResolverData)?;
                let header = probe
                    .rpc
                    .call(false, "getblockheader", json!([blockhash]))
                    .map_err(|err| WitnessResolverError::ResolverIssue(Some(witness_id), err))?;
                let height = header
                    .get("height")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32)
                    .ok_or(WitnessResolverError::InvalidResolverData)?;
                let timestamp = timestamp
                    .or_else(|| header.get("time").and_then(Value::as_i64))
                    .ok_or(WitnessResolverError::InvalidResolverData)?;
                (height, timestamp)
            }
        };
        let position = WitnessPos::bitcoin(
            std::num::NonZeroU32::new(height).ok_or(WitnessResolverError::InvalidResolverData)?,
            timestamp,
        )
        .ok_or(WitnessResolverError::InvalidResolverData)?;
        Ok(WitnessStatus::Resolved(tx, WitnessOrd::Mined(position)))
    }

    fn check_chain_net(&self, offered: ChainNet) -> Result<(), WitnessResolverError> {
        match chain_net() {
            Ok(expected) if offered == expected => Ok(()),
            _ => Err(WitnessResolverError::WrongChainNet),
        }
    }
}

impl rgbstd::validation::WitnessOrdProvider for ElectrumResolver {
    fn witness_ord(&self, witness_id: Txid) -> Result<WitnessOrd, WitnessResolverError> {
        Ok(self.resolve_witness(witness_id)?.witness_ord())
    }
}

fn parse_http(url: &str) -> Result<(String, u16), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("rpc url {url}"))?;
    let (host, port) = rest
        .split_once(':')
        .ok_or_else(|| format!("rpc url {url}"))?;
    let port = port
        .trim_end_matches('/')
        .parse()
        .map_err(|_| format!("rpc port {port}"))?;
    Ok((host.to_string(), port))
}

fn base64(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= data.len() {
        let chunk =
            ((data[index] as u32) << 16) | ((data[index + 1] as u32) << 8) | data[index + 2] as u32;
        out.push(TABLE[((chunk >> 18) & 63) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 63) as usize] as char);
        out.push(TABLE[(chunk & 63) as usize] as char);
        index += 3;
    }
    if index < data.len() {
        let mut chunk = (data[index] as u32) << 16;
        out.push(TABLE[((chunk >> 18) & 63) as usize] as char);
        if index + 1 < data.len() {
            chunk |= (data[index + 1] as u32) << 8;
            out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
            out.push(TABLE[((chunk >> 6) & 63) as usize] as char);
            out.push('=');
        } else {
            out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        }
    }
    out
}

fn decode_chunked(body: &str) -> String {
    let mut rest = body;
    let mut out = String::new();
    loop {
        let Some((size, after)) = rest.split_once("\r\n") else {
            break;
        };
        let size = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if size == 0 || after.len() < size {
            break;
        }
        out.push_str(&after[..size]);
        rest = &after[size..];
        rest = rest.strip_prefix("\r\n").unwrap_or(rest);
    }
    if out.is_empty() {
        body.trim().to_string()
    } else {
        out
    }
}
