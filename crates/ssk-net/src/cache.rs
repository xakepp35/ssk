use crate::constants::*;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{io::Write, path::Path};
pub(crate) fn read(path: &Path) -> Vec<Vec<u8>> {
    let Ok(meta) = std::fs::metadata(path) else {
        return vec![];
    };
    if meta.len() > MAX_CACHE_BYTES {
        return vec![];
    }
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<Vec<u8>>>(&b).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.len() <= MAX_SIGNED_BYTES)
        .take(MAX_PEERS)
        .collect()
}
pub(crate) fn write(path: &Path, records: Vec<Vec<u8>>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(SECRET_FILE_MODE);
    let mut file = options.open(&tmp)?;
    file.write_all(&serde_json::to_vec(&records)?)?;
    file.sync_all()?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
