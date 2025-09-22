use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use std::fs;

/// Simple bootstrap configuration structure for static bootstrap peers.
#[derive(Debug, Deserialize, Clone)]
pub struct BootstrapConfig {
    pub peers: Vec<String>,
}

impl BootstrapConfig {
    /// Load bootstrap config from a TOML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        let cfg: BootstrapConfig = toml::from_str(&data)?;
        Ok(cfg)
    }
}

/// Parse a CSV list of peers into Vec<String>
pub fn parse_peers_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}
