use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub relay_server: Option<RelayServerConfig>,
    pub world: String,
    pub seed: u64,
    pub net_hz: u32,
    pub udp_bind: SocketAddr,
    pub advertise: Option<SocketAddr>,
    pub relays: Vec<String>,
    pub seeds: Vec<String>,
    pub identity_file: PathBuf,
    pub max_peers: usize,
    pub discovery_bind: SocketAddr,
    pub discovery_advertise: Vec<String>,
    pub interest_radius: f32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            relay_server: None,
            world: "ssk-playground-v1".into(),
            seed: 42,
            net_hz: 30,
            udp_bind: "0.0.0.0:0".parse().unwrap(),
            advertise: None,
            relays: vec![],
            seeds: vec![],
            identity_file: data_dir().join("identity.key"),
            max_peers: 128,
            discovery_bind: "0.0.0.0:0".parse().unwrap(),
            discovery_advertise: vec![],
            interest_radius: 3000.,
        }
    }
}
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
        .join("ssk")
}
pub fn config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        })
        .join("ssk/config.yaml")
}
impl Config {
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let default = config_path();
        let path = path.unwrap_or(&default);
        let config = if path.exists() {
            serde_yaml::from_slice::<Self>(
                &std::fs::read(path).with_context(|| format!("read {}", path.display()))?,
            )?
        } else if path == default {
            Self::default()
        } else {
            anyhow::bail!("Config does not exist: {}", path.display())
        };
        anyhow::ensure!([30, 60].contains(&config.net_hz), "net_hz must be 30 or 60");
        anyhow::ensure!(
            config.max_peers > 0 && config.max_peers <= 128,
            "max_peers must be 1..128"
        );
        anyhow::ensure!(
            !config.world.is_empty() && config.world.len() <= 96,
            "world must contain 1..96 bytes"
        );
        anyhow::ensure!(
            config.interest_radius.is_finite()
                && (1000. ..=8192.).contains(&config.interest_radius),
            "interest_radius must be 1000..8192"
        );
        if let Some(r) = &config.relay_server {
            anyhow::ensure!(
                (1..=4096).contains(&r.max_clients),
                "relay max_clients must be 1..4096"
            );
            anyhow::ensure!(
                r.bytes_per_second > 0,
                "relay bytes_per_second must be positive"
            );
            anyhow::ensure!(
                r.https_bind.is_some() == (r.cert_file.is_some() && r.key_file.is_some()),
                "relay HTTPS requires both cert_file and key_file"
            );
            anyhow::ensure!(
                r.quic_bind.is_none() || r.https_bind.is_some(),
                "relay QUIC requires TLS"
            );
            anyhow::ensure!(
                r.quic_bind.is_none_or(|a| a.port() == 7842),
                "relay QUIC port must be 7842"
            );
        }
        Ok(config)
    }
    pub fn write_default(path: Option<&Path>) -> anyhow::Result<PathBuf> {
        use std::io::Write;
        let path = path.map(Path::to_path_buf).unwrap_or_else(config_path);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(serde_yaml::to_string(&Self::default())?.as_bytes())?;
        Ok(path)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayServerConfig {
    pub http_bind: SocketAddr,
    pub https_bind: Option<SocketAddr>,
    pub quic_bind: Option<SocketAddr>,
    pub cert_file: Option<PathBuf>,
    pub key_file: Option<PathBuf>,
    pub max_clients: usize,
    pub bytes_per_second: u32,
    pub allow: Vec<String>,
}
impl Default for RelayServerConfig {
    fn default() -> Self {
        Self {
            http_bind: "127.0.0.1:3340".parse().unwrap(),
            https_bind: None,
            quic_bind: None,
            cert_file: None,
            key_file: None,
            max_clients: 128,
            bytes_per_second: 2 * 1024 * 1024,
            allow: vec![],
        }
    }
}
pub fn load_identity(path: &Path) -> anyhow::Result<iroh::SecretKey> {
    use std::{
        io::Write,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };
    if path.exists() {
        let bytes: [u8; 32] = std::fs::read(path)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Identity must contain exactly 32 bytes"))?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        return Ok(iroh::SecretKey::from_bytes(&bytes));
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let key = iroh::SecretKey::generate();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&key.to_bytes())?;
    file.sync_all()?;
    Ok(key)
}
