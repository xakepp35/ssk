#!/usr/bin/env python3
"""Explicit, bounded Linux kernel-stack capture; never launches SSK or changes sysctls.

Run before an attended session via graphical Polkit. No GPU/NVML/ioctl calls,
SysRq, network listeners, service changes or automatic crash/reboot commands.
"""
import argparse
import datetime
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


def main():
    if sys.argv[1:] == ["_sample"]:
        print(json.dumps(snapshot()))
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seconds", type=int, default=DEFAULT_SECONDS)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "linux" or os.geteuid() != 0:
        parser.error("Use pkexec /usr/bin/python3 /absolute/tools/freeze-stacks.py --output-dir /absolute/directory")
    if not 1 <= args.seconds <= MAX_SECONDS:
        parser.error(f"seconds must be 1..{MAX_SECONDS}")
    args.output_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    path = args.output_dir / f"kernel-stacks-{stamp}-{os.getpid()}.ndjson"
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW
    deadline = time.monotonic() + args.seconds
    boot_id = read("/proc/sys/kernel/random/boot_id")
    with os.fdopen(os.open(path, flags, 0o600), "w") as output:
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
            line = json.dumps(record) + "\n"
            if output.tell() + len(line.encode()) > MAX_FILE_BYTES:
                break
            output.write(line)
            output.flush()
            os.fdatasync(output.fileno())
            time.sleep(max(0, min(SAMPLE_SECONDS - (time.monotonic() - started), deadline - time.monotonic())))
    print("Capture finished; no application or crash was started.", flush=True)


if __name__ == "__main__":
    main()
