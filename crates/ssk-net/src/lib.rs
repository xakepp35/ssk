//! Networking runs independently from simulation and rendering.
pub const PROTOCOL_VERSION: u16 = 1;

pub fn run_node() -> anyhow::Result<()> {
    anyhow::bail!("Node networking is scheduled in phase B; use `ssk play` for local play")
}
