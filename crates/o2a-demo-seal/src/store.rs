use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealRecord {
    pub name: String,
    pub outpoint: String,
    pub policy_hex: String,
    pub script_pubkey: String,
    pub paths: String,
    pub stage: String,
    pub funding_txid: String,
    pub confirmation_height: u32,
    pub value_sats: u64,
}

pub struct SealStore {
    dir: PathBuf,
}

impl SealStore {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let dir = data_dir.join("seals");
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn write(&self, record: &SealRecord) -> Result<()> {
        let path = self.dir.join(format!("{}.txt", record.name));
        if path.exists() {
            anyhow::bail!("seal record {} already exists", record.name);
        }
        fs::write(
            path,
            format!(
                "name={}\noutpoint={}\npolicy_hex={}\nscript_pubkey={}\npaths={}\nstage={}\nfunding_txid={}\nconfirmation_height={}\nvalue_sats={}\n",
                record.name,
                record.outpoint,
                record.policy_hex,
                record.script_pubkey,
                record.paths,
                record.stage,
                record.funding_txid,
                record.confirmation_height,
                record.value_sats
            ),
        )?;
        Ok(())
    }

    pub fn read(&self, name: &str) -> Result<SealRecord> {
        let text = fs::read_to_string(self.dir.join(format!("{name}.txt")))
            .with_context(|| format!("missing seal record {name}"))?;
        let mut fields = std::collections::BTreeMap::new();
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                fields.insert(key.to_owned(), value.to_owned());
            }
        }
        let get = |key: &str| {
            fields
                .get(key)
                .cloned()
                .with_context(|| format!("seal record {name} missing {key}"))
        };
        Ok(SealRecord {
            name: get("name")?,
            outpoint: get("outpoint")?,
            policy_hex: get("policy_hex")?,
            script_pubkey: get("script_pubkey")?,
            paths: get("paths")?,
            stage: get("stage")?,
            funding_txid: get("funding_txid")?,
            confirmation_height: get("confirmation_height")?.parse()?,
            value_sats: get("value_sats")?.parse()?,
        })
    }

    pub fn list(&self) -> Result<Vec<SealRecord>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let name = entry?.file_name();
            let name = name.to_string_lossy();
            if let Some(stem) = name.strip_suffix(".txt") {
                names.push(stem.to_owned());
            }
        }
        names.sort();
        names.into_iter().map(|name| self.read(&name)).collect()
    }
}
