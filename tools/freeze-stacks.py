#!/usr/bin/env python3
"""Explicit, bounded Linux kernel-stack capture; never launches SSK or changes sysctls.

Run before an attended session via graphical Polkit. No GPU/NVML/ioctl calls,
SysRq, network listeners, service changes or automatic crash/reboot commands.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

SAMPLE_SECONDS = 1
SAMPLE_TIMEOUT_SECONDS = 3
DEFAULT_SECONDS = 120
MAX_SECONDS = 900
MAX_THREADS = 48
MAX_THREAD_SCAN = 256
MAX_STACK_BYTES = 4096
MAX_FILE_BYTES = 16 * 1024 * 1024
HASH_CHUNK_BYTES = 1024 * 1024
SESSION_STAGES = ("cpu-graphics", "gpu-graphics", "gpu-worker", "offline", "network", "audio")
TARGETS = ("ssk", "gnome-shell", "Xwayland", "nvidia-modeset", "sing-box", "pipewire", "wireplumber")


def read(path, limit=MAX_STACK_BYTES):
    try:
        with Path(path).open() as source:
            return source.read(limit).strip()
    except OSError as error:
        return {"unavailable_errno": error.errno}


def snapshot():
    processes = []
    for path in Path("/proc").iterdir():
        if not path.name.isdigit():
            continue
        name = read(path / "comm")
        if isinstance(name, str) and any(name == target or name.startswith(target + "/") for target in TARGETS):
            # Prioritize display/SSK over optional network/audio processes.
            rank = 0 if name in ("ssk", "gnome-shell") or name.startswith("nvidia-modeset") else 1
            processes.append((rank, int(path.name), path, name))
    rows = []
    for _, pid, path, name in sorted(processes):
        try:
            tasks = sorted((path / "task").iterdir(), key=lambda p: int(p.name))[:MAX_THREAD_SCAN]
            # KMS thread is often the crucial one; sample it ahead of generic workers.
            tasks.sort(key=lambda p: (0 if "KMS" in str(read(p / "comm")) else 1, int(p.name)))
            for task in tasks:
                if len(rows) >= MAX_THREADS:
                    break
                # Limit each process to its leader and display-related or blocked threads.
                thread_name = read(task / "comm")
                status = read(task / "status")
                state = next((line for line in str(status).splitlines() if line.startswith("State:")), "unavailable")
                if int(task.name) != pid and not any(s in str(thread_name) for s in ("KMS", "nvidia", "ssk")) and "D (" not in state:
                    continue
                rows.append({"pid": pid, "process": name, "tid": int(task.name),
                             "thread": thread_name, "state": state,
                             "wchan": read(task / "wchan"), "kernel_stack": read(task / "stack")})
        except OSError:
            continue
    return {"threads": rows, "thread_limit": MAX_THREADS,
            "pressure": {name: read(f"/proc/pressure/{name}") for name in ("cpu", "io", "memory")},
            "uptime": read("/proc/uptime")}


def fingerprint(path):
    """Hash operator-selected build/config without recording config contents."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(HASH_CHUNK_BYTES):
            digest.update(chunk)
        size = source.tell()
    return {"path": str(path.resolve()), "bytes": size, "sha256": digest.hexdigest()}


def main():
    if sys.argv[1:] == ["_sample"]:
        print(json.dumps(snapshot()))
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seconds", type=int, default=DEFAULT_SECONDS)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--summary-file", type=Path, help="Read a bounded capture and print only the final display stacks")
    parser.add_argument("--stage", choices=SESSION_STAGES)
    parser.add_argument("--binary", type=Path, help="Optional build fingerprint; never executed")
    parser.add_argument("--app-config", type=Path, help="Optional config hash; contents are never captured")
    args = parser.parse_args()
    if sys.platform != "linux" or os.geteuid() != 0:
        parser.error("Use pkexec /usr/bin/python3 /absolute/tools/freeze-stacks.py --output-dir /absolute/directory")
    if args.summary_file:
        if args.output_dir or args.stage or args.binary or args.app_config or args.summary_file.stat().st_size > MAX_FILE_BYTES:
            parser.error("summary-file must be used alone and fit the capture size limit")
        with args.summary_file.open("rb") as source:
            data = source.read(MAX_FILE_BYTES + 1)
        if len(data) > MAX_FILE_BYTES:
            parser.error("Capture exceeded the size limit")
        records = []
        partial_tail = False
        lines = data.splitlines(keepends=True)
        for index, line in enumerate(lines):
            if not line.strip():
                continue
            try:
                records.append(json.loads(line))
            except (ValueError, UnicodeError):
                # Power loss can leave only the final record incomplete. Never
                # silently ignore malformed complete records or interior data.
                if index == len(lines) - 1 and not line.endswith(b"\n"):
                    partial_tail = True
                else:
                    parser.error(f"Malformed complete record at line {index + 1}")
        if not records or any(not isinstance(row, dict) or row.get("schema") != "ssk.kernel-capture/1" for row in records):
            parser.error("Not an SSK kernel-stack capture")
        last_threads = next((row for row in reversed(records)
                             if isinstance(row.get("sample"), dict) and "threads" in row["sample"]), None)
        threads = last_threads["sample"]["threads"] if last_threads else []
        selected = [row for row in threads if "KMS" in str(row["thread"]) or str(row["process"]).startswith("nvidia-modeset") or "D (" in row["state"]]
        print(json.dumps({"records": len(records), "first_ms": records[0]["ts_unix_ms"],
                          "last_ms": records[-1]["ts_unix_ms"], "threads": len(threads),
                          "boot_id": records[0]["boot_id"], "session": records[0].get("session"),
                          "partial_tail_ignored": partial_tail,
                          "last_thread_sample_ms": last_threads["ts_unix_ms"] if last_threads else None,
                          "sample_errors": sum("sample_error" in row.get("sample", {}) or "sample_exit" in row.get("sample", {}) for row in records),
                          "stack_read_errors": sum(isinstance(row["kernel_stack"], dict) for row in threads),
                          "display_or_blocked": [{**row, "kernel_stack": str(row["kernel_stack"]).splitlines()[:12]} for row in selected]}, indent=2))
        return
    if not args.output_dir:
        parser.error("output-dir is required for collection")
    if not 1 <= args.seconds <= MAX_SECONDS:
        parser.error(f"seconds must be 1..{MAX_SECONDS}")
    session = {"stage": args.stage, "kernel": os.uname().release,
               "seconds": args.seconds, "sample_seconds": SAMPLE_SECONDS}
    try:
        for label, path in (("binary", args.binary), ("config", args.app_config)):
            if path:
                session[label] = fingerprint(path)
    except OSError as error:
        parser.error(f"Cannot fingerprint session input: {error}")
    args.output_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = args.output_dir / f"kernel-stacks-{stamp}-{os.getpid()}.ndjson"
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW
    deadline = time.monotonic() + args.seconds
    boot_id = read("/proc/sys/kernel/random/boot_id")
    with os.fdopen(os.open(path, flags, 0o600), "w") as output:
        first = True
        print(f"Read-only stack capture for {args.seconds}s: {path}", flush=True)
        while time.monotonic() < deadline:
            started = time.monotonic()
            try:
                # A blocked procfs read cannot block the parent sample scheduler.
                child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "_sample"],
                                         stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
                try:
                    raw, _ = child.communicate(timeout=SAMPLE_TIMEOUT_SECONDS)
                except subprocess.TimeoutExpired:
                    child.kill()
                    try:
                        child.communicate(timeout=SAMPLE_SECONDS)
                    except subprocess.TimeoutExpired:
                        # Do not accumulate children if a kernel read is unkillable.
                        raise RuntimeError("Unkillable sample worker; capture stopped")
                    raise TimeoutError("Kernel stack read exceeded sample timeout")
                sample = json.loads(raw) if child.returncode == 0 else {"sample_exit": child.returncode}
            except (OSError, ValueError, TimeoutError) as error:
                sample = {"sample_error": str(error)}
            record = {"schema": "ssk.kernel-capture/1", "boot_id": boot_id,
                      "ts_unix_ms": time.time_ns() // 1_000_000,
                      "monotonic_s": time.monotonic(), "sample": sample}
            if first:
                record["session"] = session
                first = False
            line = json.dumps(record) + "\n"
            if output.tell() + len(line.encode()) > MAX_FILE_BYTES:
                break
            output.write(line)
            output.flush()
            os.fdatasync(output.fileno())
            if "session" in record:
                print("First sample persisted; collector ready. No application was started.", flush=True)
            time.sleep(max(0, min(SAMPLE_SECONDS - (time.monotonic() - started), deadline - time.monotonic())))
    print("Capture finished; no application or crash was started.", flush=True)


if __name__ == "__main__":
    main()
