use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
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
}
impl Default for Config {
    fn default() -> Self {
        Self {
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
