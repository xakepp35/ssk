use crate::constants::*;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Default)]
pub struct LinkStats {
    pub peer: String,
    pub rtt_ms: Option<f32>,
    pub jitter_ms: f32,
    pub loss: f32,
    pub tx_pps: f32,
    pub rx_pps: f32,
    pub tx_bps: f32,
    pub rx_bps: f32,
    pub queued: usize,
    pub expired: u64,
    pub age_ms: f32,
    pub direct: bool,
}
pub(crate) struct Metrics {
    pub stats: LinkStats,
    sent: VecDeque<(u64, Instant)>,
    echo: Option<(u64, Instant)>,
    last_echo: u64,
    max_seq: u64,
    mask: u64,
    window: Instant,
    tx: [u64; 2],
    rx: [u64; 2],
    last_rx: Instant,
}
impl Metrics {
    pub fn new() -> Self {
        Self {
            stats: Default::default(),
            sent: VecDeque::new(),
            echo: None,
            last_echo: 0,
            max_seq: 0,
            mask: 0,
            window: Instant::now(),
            tx: [0; 2],
            rx: [0; 2],
            last_rx: Instant::now(),
        }
    }
    pub fn header(&mut self, seq: u64) -> Vec<u8> {
        let now = Instant::now();
        self.sent.push_back((seq, now));
        while self.sent.len() > METRIC_SENT_MEMORY {
            self.sent.pop_front();
        }
        let mut b = PACKET_MAGIC.to_vec();
        b.extend(seq.to_le_bytes());
        let (echo, delay) = self
            .echo
            .map(|(s, t)| {
                (
                    s,
                    now.duration_since(t).as_micros().min(u32::MAX as u128) as u32,
                )
            })
            .unwrap_or((0, 0));
        b.extend(echo.to_le_bytes());
        b.extend(delay.to_le_bytes());
        b
    }
    pub fn receive(&mut self, b: &[u8]) -> bool {
        if b.len() < METRIC_HEADER_BYTES || !b.starts_with(PACKET_MAGIC) {
            return false;
        }
        let now = Instant::now();
        let seq = u64::from_le_bytes(b[4..12].try_into().unwrap());
        let echo = u64::from_le_bytes(b[12..20].try_into().unwrap());
        let delay = u32::from_le_bytes(b[20..24].try_into().unwrap());
        if echo > self.last_echo {
            if let Some((_, sent)) = self.sent.iter().find(|(s, _)| *s == echo) {
                let elapsed = now.duration_since(*sent);
                let delay = Duration::from_micros(delay as u64);
                if delay <= elapsed {
                    let rtt = (elapsed - delay).as_secs_f32() * 1000.;
                    let previous = self.stats.rtt_ms.unwrap_or(rtt);
                    self.stats.jitter_ms +=
                        METRIC_EWMA * ((rtt - previous).abs() - self.stats.jitter_ms);
                    self.stats.rtt_ms = Some(previous + METRIC_EWMA * (rtt - previous));
                    self.last_echo = echo;
                }
            }
        }
        if seq > self.max_seq {
            let gap = seq - self.max_seq;
            self.mask = if gap >= METRIC_LOSS_BITS {
                0
            } else {
                self.mask << gap
            };
            self.mask |= 1;
            self.max_seq = seq;
            self.echo = Some((seq, now));
        } else if self.max_seq - seq < METRIC_LOSS_BITS {
            self.mask |= 1 << (self.max_seq - seq);
        }
        let count = self.max_seq.min(METRIC_LOSS_BITS) as u32;
        let valid_mask = if count == METRIC_LOSS_BITS as u32 {
            u64::MAX
        } else {
            (1u64 << count) - 1
        };
        self.stats.loss = if count == 0 {
            0.
        } else {
            1. - (self.mask & valid_mask).count_ones() as f32 / count as f32
        };
        self.rx[0] += 1;
        self.rx[1] += b.len() as u64;
        self.last_rx = now;
        true
    }
    pub fn sent(&mut self, len: usize) {
        self.tx[0] += 1;
        self.tx[1] += len as u64;
    }
    pub fn sample(&mut self) -> LinkStats {
        let elapsed = self.window.elapsed().as_secs_f32();
        if elapsed >= METRIC_SAMPLE_SECONDS {
            self.stats.tx_pps = self.tx[0] as f32 / elapsed;
            self.stats.tx_bps = self.tx[1] as f32 / elapsed;
            self.stats.rx_pps = self.rx[0] as f32 / elapsed;
            self.stats.rx_bps = self.rx[1] as f32 / elapsed;
            self.tx = [0; 2];
            self.rx = [0; 2];
            self.window = Instant::now();
        }
        self.stats.age_ms = self.last_rx.elapsed().as_secs_f32() * 1000.;
        self.stats.clone()
    }
}
