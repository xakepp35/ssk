//! Configure Vulkan discovery before any threads or graphics libraries exist.
use ssk_client::{GraphicsAdapter, WindowConfig};

pub fn prepare(window: &WindowConfig) -> anyhow::Result<()> {
    if window.adapter != GraphicsAdapter::Software {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        isolate_lavapipe()
    }
    #[cfg(not(target_os = "linux"))]
    {
        anyhow::bail!(
            "Isolated software Vulkan is currently supported on Linux only; select hardware"
        )
    }
}

#[cfg(target_os = "linux")]
fn isolate_lavapipe() -> anyhow::Result<()> {
    use crate::constants::graphics::*;
    use std::{os::unix::process::CommandExt, path::Path, process::Command};
    // The loader ignores driver overrides for elevated/secure processes.
    // SAFETY: getauxval reads the process auxiliary vector, takes no pointers,
    // and does not mutate the environment or call a graphics driver.
    anyhow::ensure!(
        unsafe { libc::getauxval(libc::AT_SECURE) } == 0,
        "Secure execution disables Vulkan driver overrides; run SSK without file capabilities or set-id"
    );
    let status = std::fs::read_to_string("/proc/self/status")?;
    for key in ["Uid:", "Gid:"] {
        let ids: Vec<_> = status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .ok_or_else(|| anyhow::anyhow!("Missing process credentials"))?
            .split_whitespace()
            .collect();
        anyhow::ensure!(
            ids.first() != Some(&"0") && ids.first() == ids.get(1),
            "Run SSK as the regular desktop user: elevated Vulkan ignores driver isolation"
        );
    }
    let manifest = LAVAPIPE_MANIFESTS.iter().map(Path::new).find(|p| p.is_file())
        .ok_or_else(|| anyhow::anyhow!("Mesa Lavapipe is missing; install mesa-vulkan-drivers. Hardware fallback is disabled"))?;
    let manifest = manifest.canonicalize()?;
    let required = [
        ("VK_DRIVER_FILES", manifest.as_os_str()),
        // Isolate implicit overlays/device-selection layers as well as ICDs.
        (
            "VK_LOADER_LAYERS_DISABLE",
            std::ffi::OsStr::new("~implicit~"),
        ),
        ("LP_NUM_THREADS", std::ffi::OsStr::new(SOFTWARE_THREADS)),
    ];
    let removed = [
        "VK_ICD_FILENAMES",
        "VK_ADD_DRIVER_FILES",
        "VK_LOADER_DRIVERS_SELECT",
        "VK_LOADER_DRIVERS_DISABLE",
        "VK_INSTANCE_LAYERS",
        "VK_LOADER_LAYERS_ENABLE",
        "VK_LOADER_LAYERS_ALLOW",
    ];
    if required
        .iter()
        .all(|(key, value)| std::env::var_os(key).as_deref() == Some(*value))
        && removed.iter().all(|key| std::env::var_os(key).is_none())
    {
        return Ok(());
    }
    let mut child = Command::new(std::env::current_exe()?);
    child.args(std::env::args_os().skip(1));
    for (key, value) in required {
        child.env(key, value);
    }
    for key in removed {
        child.env_remove(key);
    }
    Err(child.exec().into())
}
