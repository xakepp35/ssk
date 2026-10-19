//! Process-owned structured diagnostics. Libraries only emit tracing events.
mod host;
use crate::constants::observation::*;
use serde_json::{Map, Value, json};
use std::{
    collections::{HashMap, VecDeque},
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{EnvFilter, Layer, layer::Context, prelude::*, registry::LookupSpan};

pub struct Settings {
    pub directory: PathBuf,
    pub file: Option<PathBuf>,
    pub filter: String,
    pub console: bool,
    pub sync: bool,
    pub rate_limit: u64,
}
struct Rate {
    since: Instant,
    accepted: u64,
    suppressed: u64,
}
struct Writer {
    file: File,
    path: PathBuf,
    part: u64,
    bytes: u64,
    rates: HashMap<String, Rate>,
    progress: HashMap<String, Instant>,
    operations: HashMap<String, (String, Instant)>,
    retained: VecDeque<PathBuf>,
}
struct Sink {
    writer: Mutex<Writer>,
    emergency: File,
    settings: Settings,
    run_id: String,
    boot_id: Option<String>,
    start: Instant,
    seq: AtomicU64,
    suppressed: AtomicU64,
    io_errors: AtomicU64,
    max_write_us: AtomicU64,
}
pub struct Guard {
    sink: Arc<Sink>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

fn private_file(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        #[cfg(not(unix))]
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(DIR_MODE)
                .create(parent)?;
        }
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true).read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(FILE_MODE);
    }
    let file = options.open(path)?;
    file.try_lock()
        .map_err(|e| io::Error::other(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(FILE_MODE))?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            File::open(parent)?.sync_all()?;
        }
    }
    Ok(file)
}

fn cap(text: &str) -> String {
    let mut end = text.len().min(MAX_FIELD_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let clipped = &text[..end];
    // Invitation contents are never useful to machine diagnostics.
    if clipped.contains("ssk1:") {
        return "[redacted invitation]".into();
    }
    if end < text.len() {
        format!("{clipped}…[truncated]")
    } else {
        clipped.into()
    }
}
#[derive(Default)]
struct Fields(Map<String, Value>);
impl Fields {
    fn formatted(&mut self, field: &Field, text: &str) {
        let text = cap(text);
        let structured = [
            "snapshot",
            "memory",
            "process",
            "pressure",
            "threads",
            "interfaces",
            "progress_age_ms",
            "open_operations",
            "cpu_ticks",
            "process_cpu_ticks",
            "process_io",
            "links",
        ];
        let value = if structured.contains(&field.name()) {
            serde_json::from_str(&text).unwrap_or_else(|_| json!(text))
        } else {
            json!(text)
        };
        self.put(field, value);
    }
    fn put(&mut self, field: &Field, value: Value) {
        if self.0.len() >= MAX_FIELDS {
            return;
        }
        let name = field.name().to_ascii_lowercase();
        let sensitive = [
            "password",
            "secret",
            "token",
            "authorization",
            "cookie",
            "private_key",
            "ticket",
            "invitation",
        ];
        let value = if sensitive.iter().any(|word| name.contains(word)) {
            json!("[redacted]")
        } else {
            value
        };
        self.0.insert(field.name().into(), value);
    }
}
impl Visit for Fields {
    fn record_str(&mut self, f: &Field, v: &str) {
        self.formatted(f, v);
    }
    fn record_bool(&mut self, f: &Field, v: bool) {
        self.put(f, json!(v));
    }
    fn record_i64(&mut self, f: &Field, v: i64) {
        self.put(f, json!(v));
    }
    fn record_u64(&mut self, f: &Field, v: u64) {
        self.put(f, json!(v));
    }
    fn record_f64(&mut self, f: &Field, v: f64) {
        self.put(f, json!(v));
    }
    fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
        self.formatted(f, &format!("{v:?}"));
    }
}
fn event_id(meta: &Metadata<'_>) -> String {
    if meta.name().starts_with("ssk.") {
        return meta.name().into();
    }
    let source = format!(
        "{}:{}:{}:{}",
        meta.target(),
        meta.name(),
        meta.file().unwrap_or(""),
        meta.line().unwrap_or_default()
    );
    let hash = source
        .bytes()
        .fold(HASH_OFFSET, |h, b| (h ^ b as u64).wrapping_mul(HASH_PRIME));
    format!("ext.{hash:016x}")
}
fn source(meta: &Metadata<'_>) -> Value {
    json!({"file":meta.file(),"line":meta.line(),"module":meta.module_path()})
}
fn native_thread_id()->Option<u64> {
    #[cfg(target_os="linux")] {Some(unsafe {libc::gettid()} as u64)}
    #[cfg(target_os="windows")] {
        #[link(name="kernel32")]
        unsafe extern "system" {fn GetCurrentThreadId()->u32;}
        Some(unsafe {GetCurrentThreadId()} as u64)
    }
    #[cfg(not(any(target_os="linux",target_os="windows")))] {None}
}
impl Sink {
    fn path(&self, part: u64) -> PathBuf {
        if let Some(path) = &self.settings.file {
            if part == 0 {
                return path.clone();
            }
            return path.with_file_name(format!(
                "{}.{}.{part:04}.ndjson",
                path.file_name().unwrap_or_default().to_string_lossy(),
                self.run_id
            ));
        }
        self.settings
            .directory
            .join(format!("ssk-{}-{part:04}.ndjson", self.run_id))
    }
    fn base(&self, id: &str, level: &str, kind: &str, fields: Value) -> Value {
        let thread = std::thread::current();
        json!({"schema":SCHEMA,"kind":kind,"ts_unix_ms":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
            "mono_us":self.start.elapsed().as_micros() as u64,"event_id":id,"level":level,
            "run_id":self.run_id,"trace_id":self.run_id,"boot_id":self.boot_id,"pid":std::process::id(),
            "thread_id":format!("{:?}",thread.id()),"os_thread_id":native_thread_id(),"thread_name":thread.name(),"fields":fields})
    }
    fn emit(&self, mut record: Value) {
        let began = Instant::now();
        let mut writer = self.writer.lock().unwrap_or_else(|p| p.into_inner());
        let id = record["event_id"].as_str().unwrap_or("unknown").to_owned();
        if let Some(progress_id) = record["fields"]["progress_id"]
            .as_str()
            .filter(|id| id.starts_with("ssk."))
        {
            if record["fields"]["progress_active"] == true {
                writer.progress.insert(progress_id.into(), Instant::now());
            } else if record["fields"]["progress_active"] == false {
                writer.progress.remove(progress_id);
            }
        }
        if let Some(span) = record["span_id"].as_str() {
            if record["kind"] == "span.begin" && record["fields"]["watchdog"] == true {
                writer
                    .operations
                    .insert(span.into(), (id.clone(), Instant::now()));
            } else if record["kind"] == "span.end" {
                writer.operations.remove(span);
            }
        }
        if record["level"] != "ERROR" && self.settings.rate_limit > 0 {
            if !writer.rates.contains_key(&id) && writer.rates.len() >= MAX_RATE_SITES {
                self.suppressed.fetch_add(1, Ordering::Relaxed);
                return;
            }
            let rate = writer.rates.entry(id.clone()).or_insert(Rate {
                since: Instant::now(),
                accepted: 0,
                suppressed: 0,
            });
            if rate.since.elapsed() >= Duration::from_millis(SAMPLE_MILLIS) {
                if rate.suppressed > 0 {
                    record["suppressed_since_last"] = json!(rate.suppressed);
                }
                *rate = Rate {
                    since: Instant::now(),
                    accepted: 0,
                    suppressed: 0,
                };
            }
            if rate.accepted >= self.settings.rate_limit {
                rate.suppressed += 1;
                self.suppressed.fetch_add(1, Ordering::Relaxed);
                return;
            }
            rate.accepted += 1;
        }
        // Only our bounded worker/lifecycle IDs are progress, not arbitrary data.
        if id.starts_with("ssk.") && !id.starts_with("ssk.observe.") && !id.starts_with("ssk.host.")
        {
            if id.ends_with(".sample") || id.ends_with(".begin") || id.ends_with(".ready") {
                writer.progress.insert(id.clone(), Instant::now());
            }
        }
        record["seq"] = json!(self.seq.fetch_add(1, Ordering::Relaxed));
        let Ok(mut bytes) = serde_json::to_vec(&record) else {
            self.io_errors.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if bytes.len() > MAX_RECORD_BYTES {
            record["fields"] = json!({"truncated":true,"original_bytes":bytes.len()});
            bytes = serde_json::to_vec(&record).unwrap_or_default();
        }
        bytes.push(b'\n');
        if writer.bytes + bytes.len() as u64 > MAX_FILE_BYTES {
            let next = writer.part + 1;
            let path = self.path(next);
            match private_file(&path) {
                Ok(file) => {
                    writer.file = file;
                    writer.path = path.clone();
                    writer.part = next;
                    writer.bytes = 0;
                    writer.retained.push_back(path);
                    while writer.retained.len() > RETAIN_IDLE_FILES {
                        if let Some(old) = writer.retained.pop_front() {
                            let _ = std::fs::remove_file(old);
                        }
                    }
                    self.retain();
                }
                Err(_) => {}
            }
        }
        let result = if writer.bytes + bytes.len() as u64 <= MAX_FILE_BYTES {
            writer
                .file
                .write_all(&bytes)
                .and_then(|()| writer.file.flush())
                .and_then(|()| {
                    if self.settings.sync {
                        writer.file.sync_data()
                    } else {
                        Ok(())
                    }
                })
        } else {
            Err(io::Error::other(
                "log size limit reached and rotation failed",
            ))
        };
        if let Err(error) = result {
            if self.io_errors.fetch_add(1, Ordering::Relaxed) == 0 {
                let mut failure = self.base(
                    "ssk.observe.file_failed",
                    "ERROR",
                    "log",
                    json!({"error":error.to_string(),"path":writer.path}),
                );
                failure["seq"] = json!(self.seq.fetch_add(1, Ordering::Relaxed));
                self.emergency_write(&failure);
            }
        }
        writer.bytes += bytes.len() as u64;
        // File before console: a blocked terminal must not eat the saved tail.
        if self.settings.console {
            let mut stderr = io::stderr().lock();
            if stderr
                .write_all(&bytes)
                .and_then(|()| stderr.flush())
                .is_err()
            {
                self.io_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.max_write_us
            .fetch_max(began.elapsed().as_micros() as u64, Ordering::Relaxed);
    }
    fn retain(&self) {
        let directory = self
            .settings
            .file
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or(&self.settings.directory);
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        let mut files: Vec<_> = entries
            .flatten()
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.starts_with("ssk-") && name.ends_with(".ndjson")
            })
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        files.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
        let mut idle = 0;
        for (_, path) in files {
            let Ok(file) = OpenOptions::new().read(true).write(true).open(&path) else {
                continue;
            };
            if file.try_lock().is_err() {
                continue;
            } // Never unlink another live sink.
            idle += 1;
            if idle > RETAIN_IDLE_FILES {
                drop(file);
                let _ = std::fs::remove_file(path);
            }
        }
    }
    fn panic(&self, info: &std::panic::PanicHookInfo<'_>) {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic");
        let mut record = self.base(
            "ssk.process.panic",
            "ERROR",
            "log",
            json!({"message":cap(message),
            "location":info.location().map(|p|format!("{}:{}:{}",p.file(),p.line(),p.column())),
            "backtrace":cap(&std::backtrace::Backtrace::force_capture().to_string())}),
        );
        record["seq"] = json!(self.seq.fetch_add(1, Ordering::Relaxed));
        self.emergency_write(&record);
    }
    fn emergency_write(&self, record: &Value) {
        // Dedicated FD: no dependency on a poisoned/held logger mutex.
        if let Ok(mut bytes) = serde_json::to_vec(&record) {
            bytes.push(b'\n');
            let mut file = &self.emergency;
            let _ = file.write_all(&bytes);
            let _ = file.flush();
            let _ = file.sync_data();
            if self.settings.console {
                let mut stderr = io::stderr().lock();
                let _ = stderr.write_all(&bytes);
                let _ = stderr.flush();
            }
        }
    }
}

struct SpanData {
    at: Instant,
    fields: Map<String, Value>,
}
struct JsonLayer(Arc<Sink>);
impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for JsonLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        span.extensions_mut().insert(SpanData {
            at: Instant::now(),
            fields: fields.0.clone(),
        });
        if !attrs.metadata().name().starts_with("ssk.") {
            return;
        }
        let mut record = self.0.base(
            &format!("{}.begin", event_id(attrs.metadata())),
            attrs.metadata().level().as_str(),
            "span.begin",
            json!(fields.0),
        );
        record["span_id"] = json!(format!("{:016x}", id.into_u64()));
        record["parent_span_id"] =
            json!(span.parent().map(|p| format!("{:016x}", p.id().into_u64())));
        record["source"] = source(attrs.metadata());
        record["target"] = json!(attrs.metadata().target());
        self.0.emit(record);
    }
    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            if let Some(data) = span.extensions_mut().get_mut::<SpanData>() {
                let mut fields = Fields::default();
                values.record(&mut fields);
                data.fields.extend(fields.0);
            }
        }
    }
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        let mut fields = Fields::default();
        event.record(&mut fields);
        let kind = fields
            .0
            .remove("kind")
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or("log".into());
        let mut record = self.0.base(
            &event_id(metadata),
            metadata.level().as_str(),
            &kind,
            json!(fields.0),
        );
        record["source"] = source(metadata);
        record["target"] = json!(metadata.target());
        if let Some(span) = ctx.event_span(event) {
            record["span_id"] = json!(format!("{:016x}", span.id().into_u64()));
            record["parent_span_id"] =
                json!(span.parent().map(|p| format!("{:016x}", p.id().into_u64())));
            record["span_path"] = json!(
                span.scope()
                    .from_root()
                    .map(|s| s.metadata().name())
                    .collect::<Vec<_>>()
            );
        }
        self.0.emit(record);
    }
    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        if span
            .extensions()
            .get::<SpanData>()
            .is_some_and(|s| s.fields.get("watchdog") == Some(&Value::Bool(true)))
        {
            self.0
                .writer
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .operations
                .entry(format!("{:016x}", id.into_u64()))
                .or_insert((
                    format!("{}.begin", event_id(span.metadata())),
                    Instant::now(),
                ));
        }
    }
    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        if span
            .extensions()
            .get::<SpanData>()
            .is_some_and(|s| s.fields.get("watchdog") == Some(&Value::Bool(true)))
        {
            self.0
                .writer
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .operations
                .remove(&format!("{:016x}", id.into_u64()));
        }
    }
    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else {
            return;
        };
        if !span.metadata().name().starts_with("ssk.") {
            return;
        }
        let extensions = span.extensions();
        let Some(data) = extensions.get::<SpanData>() else {
            return;
        };
        let mut fields = data.fields.clone();
        fields.insert(
            "duration_us".into(),
            json!(data.at.elapsed().as_micros() as u64),
        );
        let mut record = self.0.base(
            &format!("{}.end", event_id(span.metadata())),
            span.metadata().level().as_str(),
            "span.end",
            json!(fields),
        );
        record["span_id"] = json!(format!("{:016x}", id.into_u64()));
        record["target"] = json!(span.metadata().target());
        record["source"] = source(span.metadata());
        record["parent_span_id"] =
            json!(span.parent().map(|p| format!("{:016x}", p.id().into_u64())));
        self.0.emit(record);
    }
}

pub fn install(settings: Settings) -> anyhow::Result<Guard> {
    let filter = EnvFilter::builder()
        .with_regex(false)
        .parse(&settings.filter)?;
    let mut nonce = [0u8; ID_BYTES];
    getrandom::fill(&mut nonce)
        .map_err(|e| anyhow::anyhow!("log run-id generation failed: {e}"))?;
    let run_id = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let path = settings
        .file
        .clone()
        .unwrap_or_else(|| settings.directory.join(format!("ssk-{run_id}-0000.ndjson")));
    let file = private_file(&path)?;
    let bytes = file.metadata()?.len();
    let emergency = private_file(&path.with_file_name(format!("ssk-{run_id}-panic.ndjson")))?;
    let sink = Arc::new(Sink {
        writer: Mutex::new(Writer {
            file,
            path: path.clone(),
            part: 0,
            bytes,
            rates: HashMap::new(),
            progress: HashMap::new(),
            operations: HashMap::new(),
            retained: VecDeque::from([path.clone()]),
        }),
        emergency,
        settings,
        run_id,
        boot_id: std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .ok()
            .map(|s| s.trim().into()),
        start: Instant::now(),
        seq: AtomicU64::new(0),
        suppressed: AtomicU64::new(0),
        io_errors: AtomicU64::new(0),
        max_write_us: AtomicU64::new(0),
    });
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(filter)
            .with(JsonLayer(sink.clone())),
    )?;
    tracing_log::LogTracer::init()?;
    let panic_sink = sink.clone();
    std::panic::set_hook(Box::new(move |info| panic_sink.panic(info)));
    sink.retain();
    tracing::info!(name:"ssk.observe.ready",log_file=%path.display(),filter=%sink.settings.filter,sync_each=sink.settings.sync,
        console=sink.settings.console,rate_limit=sink.settings.rate_limit,max_file_bytes=MAX_FILE_BYTES,retain_idle_files=RETAIN_IDLE_FILES,"Structured diagnostics ready");
    let stop = Arc::new(AtomicBool::new(false));
    let thread = host::start(sink.clone(), stop.clone())?;
    Ok(Guard {
        sink,
        stop,
        thread: Some(thread),
    })
}
impl Drop for Guard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
        tracing::info!(name:"ssk.observe.stopped",suppressed_total=self.sink.suppressed.load(Ordering::Relaxed),
            io_errors=self.sink.io_errors.load(Ordering::Relaxed),max_write_us=self.sink.max_write_us.load(Ordering::Relaxed),"Diagnostics shutdown");
    }
}

pub fn snapshot() {
    host::snapshot();
}
