pub const DEFAULT_CONFIG_FILE: &str = "ssk.yml";
pub const DEFAULT_LAN_SERVER: &str = "192.168.50.155:4202";
pub const MAX_CONFIG_BYTES: u64 = 64 * 1024;

pub mod observation {
    pub const SCHEMA: &str = "ssk.obs/1";
    pub const LOG_DIR: &str = "runtime/logs";
    pub const FILTER: &str = "warn,ssk=debug,ssk_client=debug,ssk_net=debug";
    pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
    pub const RETAIN_IDLE_FILES: usize = 8;
    pub const MAX_FIELD_BYTES: usize = 32 * 1024;
    pub const MAX_FIELDS: usize = 64;
    pub const MAX_RECORD_BYTES: usize = 128 * 1024;
    pub const MAX_RATE_SITES: usize = 4096;
    pub const SITE_RECORDS_PER_SEC: u64 = 20;
    pub const SAMPLE_MILLIS: u64 = 1000;
    pub const STALL_MILLIS: u128 = 5000;
    pub const MAX_THREAD_ROWS: usize = 128;
    pub const MAX_INTERFACE_ROWS: usize = 64;
    pub const MAX_INTERFACE_SCAN: usize = 1024;
    pub const PROC_UTIME_INDEX: usize = 11;
    pub const PROC_STIME_INDEX: usize = 12;
    pub const CPU_TICK_NAMES: [&str; 10] = [
        "user",
        "nice",
        "system",
        "idle",
        "iowait",
        "irq",
        "softirq",
        "steal",
        "guest",
        "guest_nice",
    ];
    pub const FILE_MODE: u32 = 0o600;
    pub const DIR_MODE: u32 = 0o700;
    pub const ID_BYTES: usize = 16;
    pub const HASH_OFFSET: u64 = 0xcbf29ce484222325;
    pub const HASH_PRIME: u64 = 0x100000001b3;
}
