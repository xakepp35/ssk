//! Read-only proc/sysfs monitoring. Never queries GPU ioctls or opens devices.
use super::*;

#[cfg(target_os = "linux")]
fn text(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().chars().take(MAX_FIELD_BYTES).collect())
}
#[cfg(target_os = "linux")]
fn key_values(path: &str, allowed: &[&str]) -> Value {
    let Some(data) = text(path) else {
        return Value::Null;
    };
    let mut out = Map::new();
    for line in data.lines() {
        let mut parts = line.split_whitespace();
        if let Some(key) = parts.next() {
            let key = key.trim_end_matches(':');
            if allowed.is_empty() || allowed.contains(&key) {
                let parts = parts.collect::<Vec<_>>();
                let value = parts.first().and_then(|v| v.parse::<u64>().ok());
                out.insert(
                    key.into(),
                    match value {
                        Some(value) if parts.len() == 1 => json!(value),
                        Some(value) if parts.len() == 2 => json!({"value":value,"unit":parts[1]}),
                        _ => json!(parts.join(" ")),
                    },
                );
            }
        }
    }
    json!(out)
}
#[cfg(target_os = "linux")]
fn number(path: impl AsRef<Path>) -> Option<u64> {
    text(path)?.parse().ok()
}
#[cfg(target_os = "linux")]
fn pressure(path: &str) -> Value {
    let Some(data) = text(path) else {
        return Value::Null;
    };
    let mut out = Map::new();
    for line in data.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        let fields = parts
            .filter_map(|s| {
                let (k, v) = s.split_once('=')?;
                Some((
                    k.to_owned(),
                    if k == "total" {
                        json!(v.parse::<u64>().ok())
                    } else {
                        json!(v.parse::<f64>().ok())
                    },
                ))
            })
            .collect::<Map<_, _>>();
        out.insert(name.into(), json!(fields));
    }
    json!(out)
}
pub(super) fn snapshot() {
    let mut fields = json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,
        "version":env!("CARGO_PKG_VERSION"),"revision":option_env!("SSK_BUILD_REVISION"),
        "logic_hz":ssk_sim::TICK_PER_SEC,"ticks_per_packet":ssk_sim::TICK_PER_PACKET,
        "rules_id":ssk_net::constants::RULES_ID});
    #[cfg(target_os = "linux")]
    {
        fields["kernel"] = json!(text("/proc/sys/kernel/osrelease"));
        fields["nvidia_module"] = json!(text("/proc/driver/nvidia/version"));
        fields["session_type"] = json!(std::env::var("XDG_SESSION_TYPE").ok());
        fields["watchdog"] = json!({"enabled":text("/proc/sys/kernel/watchdog"),"nmi":text("/proc/sys/kernel/nmi_watchdog"),
            "hardlockup_panic":text("/proc/sys/kernel/hardlockup_panic"),"softlockup_panic":text("/proc/sys/kernel/softlockup_panic"),
            "kdump_loaded":text("/sys/kernel/kexec_crash_loaded")});
        fields["sampling"] = json!({"gpu":"module/sysfs only; no driver calls","kernel_stacks":"not collected without privileges"});
    }
    #[cfg(not(target_os = "linux"))]
    {
        fields["host_metrics"] = json!("Linux proc/sysfs metrics unavailable on this OS");
    }
    tracing::info!(name:"ssk.host.baseline",snapshot=%fields,"Host baseline (allowlisted)");
    sample();
}
fn sample() {
    #[cfg(target_os = "linux")]
    {
        let memory = key_values(
            "/proc/meminfo",
            &[
                "MemTotal",
                "MemAvailable",
                "SwapTotal",
                "SwapFree",
                "Dirty",
                "Writeback",
            ],
        );
        let process = key_values(
            "/proc/self/status",
            &[
                "State",
                "VmRSS",
                "VmSize",
                "Threads",
                "voluntary_ctxt_switches",
                "nonvoluntary_ctxt_switches",
            ],
        );
        let pressure = json!({"cpu":pressure("/proc/pressure/cpu"),"memory":pressure("/proc/pressure/memory"),"io":pressure("/proc/pressure/io")});
        let mut threads = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/proc/self/task") {
            for entry in entries.flatten().take(MAX_THREAD_ROWS) {
                let path = entry.path();
                threads.push(json!({"tid":entry.file_name().to_string_lossy(),"name":text(path.join("comm")),
                "state":key_values(path.join("status").to_str().unwrap_or(""),&["State"]),"wchan":text(path.join("wchan")).filter(|s|s!="0")}));
            }
        }
        let mut interfaces = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            let mut entries = entries
                .flatten()
                .take(MAX_INTERFACE_SCAN)
                .collect::<Vec<_>>();
            entries.sort_by_key(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let priority = if name.contains("tun") {
                    0
                } else if name.starts_with("veth") || name.starts_with("br-") {
                    2
                } else {
                    1
                };
                (priority, name)
            });
            for entry in entries.into_iter().take(MAX_INTERFACE_ROWS) {
                let path = entry.path();
                interfaces.push(json!({"name":entry.file_name().to_string_lossy(),"state":text(path.join("operstate")),
                "mtu":number(path.join("mtu")),"rx_bytes":number(path.join("statistics/rx_bytes")),"tx_bytes":number(path.join("statistics/tx_bytes")),
                "rx_dropped":number(path.join("statistics/rx_dropped")),"tx_dropped":number(path.join("statistics/tx_dropped"))}));
            }
        }
        let cpu_ticks = text("/proc/stat")
            .and_then(|s| s.lines().next().map(str::to_owned))
            .map(|s| {
                s.split_whitespace()
                    .skip(1)
                    .zip(CPU_TICK_NAMES)
                    .filter_map(|(v, k)| Some((k.into(), json!(v.parse::<u64>().ok()?))))
                    .collect::<Map<_, _>>()
            });
        let process_cpu_ticks = text("/proc/self/stat")
            .and_then(|s| s.rsplit_once(')').map(|(_, s)| s.to_owned()))
            .map(|s| {
                let fields = s.split_whitespace().collect::<Vec<_>>();
                json!({"user":fields.get(PROC_UTIME_INDEX).and_then(|s|s.parse::<u64>().ok()),
                "system":fields.get(PROC_STIME_INDEX).and_then(|s|s.parse::<u64>().ok())})
            });
        tracing::debug!(name:"ssk.host.sample",kind="metric",memory=%memory,process=%process,pressure=%pressure,
            loadavg=?text("/proc/loadavg"),cpu_ticks=%json!(cpu_ticks),
            process_cpu_ticks=%json!(process_cpu_ticks),process_io=%key_values("/proc/self/io",&[]),
            thread_rows_capped=process["Threads"].as_u64().is_some_and(|n|n>MAX_THREAD_ROWS as u64),interface_row_limit=MAX_INTERFACE_ROWS,
            threads=%json!(threads),interfaces=%json!(interfaces),"Read-only Linux host/process sample");
    }
}
pub(super) fn start(
    sink: Arc<Sink>,
    stop: Arc<AtomicBool>,
) -> io::Result<std::thread::JoinHandle<()>> {
    snapshot();
    std::thread::Builder::new().name("ssk-observe".into()).spawn(move || {
        while !stop.load(Ordering::Acquire) {
            std::thread::park_timeout(Duration::from_millis(SAMPLE_MILLIS));
            if stop.load(Ordering::Acquire){break;}
            sample();
            // try_lock keeps the monitor from waiting on a blocked sink here.
            let progress=sink.writer.try_lock().ok().map(|w|(
                w.progress.iter().filter(|(id,_)|id.ends_with(".sample")).map(|(id,t)|(id.clone(),t.elapsed().as_millis() as u64)).collect::<HashMap<_,_>>(),
                w.operations.iter().map(|(id,(name,at))|json!({"span_id":id,"event_id":name,"age_ms":at.elapsed().as_millis() as u64})).collect::<Vec<_>>(),w.path.clone()));
            if let Some((progress,operations,path))=progress {
                let stalled=progress.values().any(|age|*age as u128>STALL_MILLIS)||operations.iter().any(|v|v["age_ms"].as_u64().is_some_and(|age|age as u128>STALL_MILLIS));
                tracing::debug!(name:"ssk.observe.sample",kind="metric",progress_age_ms=%json!(progress),
                    open_operations=%json!(operations),active_file=%path.display(),
                    suppressed_total=sink.suppressed.load(Ordering::Relaxed),io_errors=sink.io_errors.load(Ordering::Relaxed),
                    max_write_us=sink.max_write_us.load(Ordering::Relaxed),"Logger and worker progress sample");
                if stalled {tracing::warn!(name:"ssk.observe.progress.stalled",threshold_ms=STALL_MILLIS as u64,"Worker progress sample overdue; inspect thread/wait snapshots");}
            }
        }
    })
}
