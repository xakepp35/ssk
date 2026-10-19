//! Application configuration composes presentation and transport settings.
use crate::constants::*;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Offline,
    #[default]
    Host,
    Join,
}

#[derive(Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub mode: Mode,
    pub connect: Option<String>,
    pub window: ssk_client::WindowConfig,
    pub audio: ssk_client::AudioConfig,
    pub network: ssk_net::Config,
}
impl Default for Config {
    fn default() -> Self {
        let mut network = ssk_net::Config::default();
        network.identity_file = "runtime/identity.key".into();
        network.contact_cache = None;
        network.udp_bind = ssk_net::constants::DEFAULT_GAME_BIND.parse().unwrap();
        network.discovery_bind = ssk_net::constants::DEFAULT_DISCOVERY_BIND.parse().unwrap();
        Self {
            mode: Mode::Host,
            connect: Some(DEFAULT_LAN_SERVER.into()),
            window: Default::default(),
            audio: Default::default(),
            network,
        }
    }
}
impl Config {
    pub fn load(explicit: Option<&Path>) -> anyhow::Result<Self> {
        let path = if let Some(path) = explicit {
            path.to_path_buf()
        } else if Path::new(DEFAULT_CONFIG_FILE).exists() {
            DEFAULT_CONFIG_FILE.into()
        } else {
            let beside_exe = std::env::current_exe()?.with_file_name(DEFAULT_CONFIG_FILE);
            if beside_exe.exists() {
                beside_exe
            } else {
                DEFAULT_CONFIG_FILE.into()
            }
        };
        let mut config = if path.exists() {
            anyhow::ensure!(
                std::fs::metadata(&path)?.len() <= MAX_CONFIG_BYTES,
                "Config too large"
            );
            serde_yaml::from_slice::<Self>(&std::fs::read(&path)?)
                .with_context(|| format!("Parse {}", path.display()))?
        } else if explicit.is_some() {
            anyhow::bail!("Config does not exist: {}", path.display());
        } else {
            Self::default()
        };
        // Resolve files against the selected config, including double-click launches.
        let base = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        if config.network.identity_file.is_relative() {
            config.network.identity_file = base.join(&config.network.identity_file);
        }
        if let Some(cache) = &mut config.network.contact_cache {
            if cache.is_relative() {
                *cache = base.join(&*cache);
            }
        }
        if let Some(relay) = &mut config.network.relay_server {
            for file in [&mut relay.cert_file, &mut relay.key_file]
                .into_iter()
                .flatten()
            {
                if file.is_relative() {
                    *file = base.join(&*file);
                }
            }
        }
        config.network.validate()?;
        anyhow::ensure!(
            ssk_client::constants::WINDOW_WIDTH_RANGE.contains(&config.window.width)
                && ssk_client::constants::WINDOW_HEIGHT_RANGE.contains(&config.window.height),
            "Window dimensions out of range"
        );
        Ok(config)
    }
    pub fn write_default(path: Option<&Path>) -> anyhow::Result<PathBuf> {
        use std::io::Write;
        let path = path.unwrap_or(Path::new(DEFAULT_CONFIG_FILE));
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(serde_yaml::to_string(&Self::default())?.as_bytes())?;
        Ok(path.to_path_buf())
    }
}
