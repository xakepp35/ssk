#!/usr/bin/env python3
# UNFINISHED DRAFT: observability work paused for graphics migration.
"""Local NDJSON host watcher, explicit child stderr capture and offline reconstruction."""
import argparse
from collections import Counter, defaultdict, deque
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import threading
import time
import traceback
import secrets

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = "ssk.obs/1"
SAMPLE_SECONDS = 1
DISCOVERY_SECONDS = 5
MAX_FILE_BYTES = 16 * 1024 * 1024
KEEP_FILES = 8
MAX_LINE_BYTES = 128 * 1024
MAX_BUFFER_BYTES = 1024 * 1024
MAX_MESSAGE_CHARS = 8192
MAX_PIDS = 16
MAX_PID_SCAN = 4096
MAX_THREADS = 64
MAX_STACKS = 8
REGULAR_KERNEL_RECORDS_PER_SEC = 20
FILE_MODE = 0o600
DIR_MODE = 0o700
PROCESS_NAMES = {"ssk", "gnome-shell", "Xwayland", "pipewire", "wireplumber", "sing-box", "xray", "v2rayN", "NetworkManager"}
CRITICAL = ("Call Trace:", "RIP:", "NMI", "hard LOCKUP", "soft lockup", "Kernel panic", "kernel BUG", "general protection fault")
KEYWORDS = ("nvidia", "NVRM", "Xid", "usb", "URB", "tun", "watchdog", "lockup", "panic", "oom", "I/O error")


def text(path, limit=MAX_MESSAGE_CHARS):
    try:
        with open(path) as source:
            return source.read(limit).strip()
    except (OSError, UnicodeError):
        return None


def safe_message(message):
    if "ssk1:" in message:
        return "[invitation omitted]"
    if "Command line:" in message or "Kernel command line:" in message:
        return "[kernel command line omitted]"
    return message[:MAX_MESSAGE_CHARS]


def write_all(fd, data):
    remaining = memoryview(data)
    while remaining:
        remaining = remaining[os.write(fd, remaining):]


class Sink:
    def __init__(self, directory):
        directory.mkdir(mode=DIR_MODE, parents=True, exist_ok=True)
        self.directory = directory
        self.run_id = secrets.token_hex(16)
        self.boot_id = text("/proc/sys/kernel/random/boot_id")
        self.started = time.monotonic_ns()
        self.seq = 0
        self.part = 0
        self.size = 0
        self.fd = None
        self.files = deque()
        self.rotate()

    def rotate(self):
        path = self.directory / f"host-{self.run_id}-{self.part:04}.ndjson"
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_APPEND, FILE_MODE)
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if self.fd is not None:
            os.close(self.fd)
        self.fd = fd
        self.part += 1
        self.size = 0
        self.files.append(path)
        while len(self.files) > KEEP_FILES:
            self.files.popleft().unlink(missing_ok=True)
        # Do not remove files owned by another live watcher.
        idle = 0
        for old in sorted(self.directory.glob("host-*.ndjson"), key=lambda p: p.stat().st_mtime_ns, reverse=True):
            try:
                with old.open("r+") as handle:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    idle += 1
                if idle > KEEP_FILES:
                    old.unlink(missing_ok=True)
            except OSError:
                continue
        directory_fd = os.open(self.directory, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)

    def emit(self, event_id, fields=None, level="DEBUG", kind="log", **extra):
        record = dict(schema=SCHEMA, kind=kind, ts_unix_ms=time.time_ns() // 1_000_000,
                      mono_us=(time.monotonic_ns() - self.started) // 1000,
                      run_id=self.run_id, trace_id=self.run_id, boot_id=self.boot_id,
                      pid=os.getpid(), thread_id=str(threading.get_ident()),
                      os_thread_id=threading.get_native_id(), thread_name=threading.current_thread().name,
                      event_id=event_id, level=level, target="ssk.host.observer",
                      source={"file":str(Path(__file__).name)}, fields=fields or {}, **extra)
        self.write(record)

    def write(self, record):
        # Original app records retain their process/run/seq identity.
        if record.get("run_id") == self.run_id:
            record["seq"] = self.seq
            self.seq += 1
        encoded = json.dumps(record, ensure_ascii=False, separators=(",", ":"))
        data = (encoded + "\n").encode()
        if len(data) > MAX_LINE_BYTES:
            record = dict(record, fields={"truncated":True,"original_bytes":len(data)})
            encoded = json.dumps(record, ensure_ascii=False, separators=(",", ":"))
            data = (encoded + "\n").encode()
        if self.size + len(data) > MAX_FILE_BYTES:
            self.rotate()
        write_all(self.fd, data)
        os.fdatasync(self.fd)
        self.size += len(data)
        sys.stderr.write(encoded + "\n")
        sys.stderr.flush()

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None


def status(path):
    data = text(path)
    if data is None:
        return None
    allowed = {"State", "VmRSS", "VmSize", "Threads"}
    return {k:v.strip() for line in data.splitlines() if ":" in line for k,v in [line.split(":", 1)] if k in allowed}


def discover(explicit):
    if explicit:
        return explicit
    candidates = []
    for entry in list(Path("/proc").iterdir())[:MAX_PID_SCAN]:
        if not entry.name.isdecimal():
            continue
        name = text(entry / "comm")
        if name in PROCESS_NAMES:
            candidates.append((name != "ssk", name, int(entry.name)))
    return [pid for _,_,pid in sorted(candidates)[:MAX_PIDS]]


def process_sample(pid, stacks):
    base = Path("/proc") / str(pid)
    rows = []
    stack_reads = 0
    try:
        tasks = sorted((base / "task").iterdir(), key=lambda p:int(p.name))
    except OSError as error:
        return {"pid":pid,"available":False,"errno":error.errno}
    # Put the UI/game/network workers ahead of anonymous GPU/library helpers.
    tasks.sort(key=lambda p:not (text(p / "comm") or "").startswith("ssk"))
    for task in tasks[:MAX_THREADS]:
        row = {"tid":int(task.name),"name":text(task / "comm"),"status":status(task / "status")}
        row["wchan"] = text(task / "wchan")
        if row["wchan"] == "0":
            row["wchan"] = None
        if stacks and stack_reads < MAX_STACKS:
            stack_reads += 1
            try:
                with (task / "stack").open() as source:
                    row["kernel_stack"] = source.read(MAX_MESSAGE_CHARS)
            except OSError as error:
                row["kernel_stack_unavailable_errno"] = error.errno
        rows.append(row)
    return {"pid":pid,"name":text(base / "comm"),"status":status(base / "status"),
            "threads":rows,"threads_total":len(tasks),"threads_capped":len(tasks)>MAX_THREADS,
            "stack_capture_requested":stacks,"stack_read_limit":MAX_STACKS}


def stop(process):
    if process is None or process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass


def collect(args):
    sink = Sink(args.directory)
    journal = child = None
    sink.emit("ssk.host.observer.begin", {"kernel":text("/proc/sys/kernel/osrelease"),
              "nvidia_module":text("/proc/driver/nvidia/version"),"sample_seconds":SAMPLE_SECONDS,
              "stacks_requested":args.stacks,"note":"No GPU/device calls, settings changes or automatic crash/probe"}, "INFO")
    try:
        journal = subprocess.Popen(["journalctl", "--boot", "--follow", "--output=json", "--no-pager", "--lines=0", "_TRANSPORT=kernel"],
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0)
        if args.action == "run":
            command = args.command
            if command and command[0] == "--":
                command = command[1:]
            if not command:
                raise ValueError("run requires an explicit command after --; no program launched")
            child = subprocess.Popen(command, stderr=subprocess.PIPE, bufsize=0)
            sink.emit("ssk.host.child.begin", {"child_pid":child.pid,"executable":Path(command[0]).name,"arguments":"omitted"}, "INFO")
        buffers = {}
        with selectors.DefaultSelector() as selector:
            for name,pipe in (("kernel",journal.stdout),("journal_notice",journal.stderr),("child",child.stderr if child else None)):
                if pipe is not None:
                    selector.register(pipe, selectors.EVENT_READ, name)
                    buffers[name] = b""
            next_sample = next_discovery = time.monotonic()
            pids = []
            window_at = time.monotonic()
            kernel_records = kernel_dropped = 0
            keywords = Counter()
            while True:
                now = time.monotonic()
                if now >= next_discovery:
                    pids = discover(args.pid + ([child.pid] if child else []))
                    next_discovery = now + DISCOVERY_SECONDS
                if now >= next_sample:
                    sink.emit("ssk.host.process.sample", {"processes":[process_sample(p,args.stacks) for p in pids],
                              "pressure":{k:text(f"/proc/pressure/{k}") for k in ("cpu","memory","io")},
                              "kernel_records_suppressed":kernel_dropped,"kernel_keywords":dict(keywords)}, kind="metric")
                    next_sample = now + SAMPLE_SECONDS
                if now - window_at >= SAMPLE_SECONDS:
                    kernel_records = 0
                    window_at = now
                for key,_ in selector.select(timeout=min(SAMPLE_SECONDS, max(0, next_sample-time.monotonic()))):
                    name = key.data
                    data = os.read(key.fileobj.fileno(), MAX_LINE_BYTES)
                    if not data:
                        selector.unregister(key.fileobj)
                        if buffers[name]:
                            sink.emit("ssk.host.stream.partial", {"stream":name,"bytes":len(buffers[name])}, "WARN")
                        continue
                    buffers[name] += data
                    lines = buffers[name].split(b"\n")
                    buffers[name] = lines.pop()
                    if len(buffers[name]) > MAX_BUFFER_BYTES:
                        sink.emit("ssk.host.stream.capped", {"stream":name,"bytes":len(buffers[name])}, "WARN")
                        buffers[name] = b""
                    for line in lines:
                        if name == "journal_notice":
                            sink.emit("ssk.host.journal.notice", {"message":safe_message(line.decode(errors="replace"))}, "WARN")
                            continue
                        try:
                            record = json.loads(line)
                        except (ValueError, UnicodeError):
                            record = None
                        if name == "child":
                            if isinstance(record, dict) and record.get("schema") == SCHEMA:
                                sink.write(record)
                            else:
                                message = safe_message(line.decode(errors="replace"))
                                sink.emit("ssk.native.stderr", {"child_pid":child.pid,"message":message,
                                          "origin_callsite":"unavailable; native stderr bridge", "message_hash":hashlib.sha256(line).hexdigest()})
                        elif isinstance(record, dict):
                            message = record.get("MESSAGE", "")
                            if not isinstance(message, str):
                                message = "[non-text journal entry omitted]"
                            for keyword in KEYWORDS:
                                if keyword.lower() in message.lower():
                                    keywords[keyword] += 1
                            critical = any(word.lower() in message.lower() for word in CRITICAL) or "+0x" in message
                            if not critical and kernel_records >= REGULAR_KERNEL_RECORDS_PER_SEC:
                                kernel_dropped += 1
                                continue
                            kernel_records += 1
                            sink.emit("ssk.host.journal.kernel", {"message":safe_message(message),"priority":record.get("PRIORITY"),
                                      "journal_cursor":record.get("__CURSOR"),"journal_ts_unix_us":record.get("__REALTIME_TIMESTAMP"),
                                      "journal_mono_us":record.get("__MONOTONIC_TIMESTAMP")}, "WARN" if critical else "DEBUG")
                if journal.poll() is not None and not any(k.data in ("kernel","journal_notice") for k in selector.get_map().values()):
                    sink.emit("ssk.host.journal.end", {"returncode":journal.returncode,"kernel_capture_available":False}, "WARN")
                    journal = None
                if journal is None:
                    # Keep process samples alive, without repeatedly announcing journal failure.
                    break
                if child and child.poll() is not None and not any(k.data=="child" for k in selector.get_map().values()):
                    sink.emit("ssk.host.child.end", {"child_pid":child.pid,"returncode":child.returncode}, "INFO")
                    return child.returncode
    except KeyboardInterrupt:
        sink.emit("ssk.host.observer.interrupted", level="INFO")
    except Exception as error:
        sink.emit("ssk.host.observer.failed", {"error":safe_message(str(error)),"backtrace":safe_message(traceback.format_exc())}, "ERROR")
        return 1
    finally:
        stop(child)
        stop(journal)
        sink.emit("ssk.host.observer.end", level="INFO")
        sink.close()
    return 0


def reconstruct(args):
    runs = defaultdict(lambda:{"records":0,"levels":Counter(),"events":Counter(),"open_spans":{},"tail":deque(maxlen=args.tail),"metrics":{},"errors":deque(maxlen=args.tail)})
    malformed = 0
    paths = set()
    for path in args.paths:
        paths.update(path.rglob("*.ndjson") if path.is_dir() else [path])
    for path in sorted(paths, key=lambda p:p.stat().st_mtime_ns):
        with path.open("rb") as source:
            for line in source:
                try:
                    if len(line) > MAX_LINE_BYTES:
                        raise ValueError("oversize")
                    record = json.loads(line)
                    if record.get("schema") != SCHEMA:
                        raise ValueError("unknown schema")
                except (ValueError, UnicodeError, AttributeError):
                    malformed += 1
                    continue
                run = runs[record.get("run_id", "unknown")]
                run["records"] += 1
                run["levels"][record.get("level")] += 1
                run["events"][record.get("event_id")] += 1
                run["pid"] = record.get("pid")
                run["boot_id"] = record.get("boot_id")
                if record.get("kind") == "span.begin":
                    run["open_spans"][record.get("span_id")] = record
                if record.get("kind") == "span.end":
                    run["open_spans"].pop(record.get("span_id"), None)
                if record.get("kind") == "metric":
                    run["metrics"][record.get("event_id")] = record
                if record.get("level") in ("ERROR", "WARN"):
                    run["errors"].append(record)
                run["tail"].append(record)
    for run_id,run in runs.items():
        # Capture wrappers may contain copies of app records from other files;
        # this is a forensic summary, not a deduplicated metrics database.
        output = dict(run, run_id=run_id, open_spans=list(run["open_spans"].values()),
                      tail=list(run["tail"]), errors=list(run["errors"]))
        print(json.dumps(output, ensure_ascii=False, indent=2))
    print(json.dumps({"malformed_or_partial_records":malformed,"note":"Unclosed spans in the retained window are leads, not proof of a hang; wrapped app records may be duplicated."}), file=sys.stderr)
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    for action in ("watch", "run"):
        p = sub.add_parser(action)
        p.add_argument("--directory", type=Path, default=ROOT / "runtime/host-logs")
        p.add_argument("--pid", type=int, action="append", default=[])
        p.add_argument("--stacks", action="store_true", help="attempt bounded /proc thread stacks; no privilege elevation")
        if action == "run":
            p.add_argument("command", nargs=argparse.REMAINDER)
    summary = sub.add_parser("summary")
    summary.add_argument("paths", type=Path, nargs="+", help="NDJSON files or log directories")
    summary.add_argument("--tail", type=int, default=20)
    args = parser.parse_args()
    if args.action == "summary":
        if args.tail < 1 or args.tail > 1000:
            parser.error("tail must be between 1 and 1000")
        return reconstruct(args)
    if sys.platform != "linux":
        parser.error("host collection uses Linux proc/journal; summary works with portable NDJSON")
    if any(pid < 1 for pid in args.pid):
        parser.error("PID must be positive")
    return collect(args)


if __name__ == "__main__":
    raise SystemExit(main())
