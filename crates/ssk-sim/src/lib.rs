//! Platform independent, fixed frequency game simulation.
pub const TICK_HZ: u64 = 240;
pub const DT: f32 = 1.0 / TICK_HZ as f32;

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum ShipClass {
    Spark,
    #[default]
    Cadet,
    Thunder,
}
