#!/usr/bin/env python3
"""Explicit, bounded Linux graphics probe; never starts a game or changes the OS."""
import argparse
import datetime
import json
import os
from pathlib import Path
import platform
import selectors
import shutil
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_TIMEOUT_SECONDS = 20
STOP_GRACE_SECONDS = 2
MAX_TRACE_BYTES = 16 * 1024 * 1024
READ_CHUNK_BYTES = 64 * 1024
FILE_MODE = 0o600
DIR_MODE = 0o700
SYSCALLS = (
    "openat,close,ioctl,mmap,munmap,mprotect,brk,"
    "poll,ppoll,select,pselect6,epoll_create1,epoll_ctl,epoll_wait,epoll_pwait,"
    "futex,futex_waitv,clone,clone3,socket,connect,"
    "read,readv,write,writev,recvmsg,sendmsg,recvfrom,sendto,"
    "io_uring_setup,bpf,clock_nanosleep"
)
# Do not decode buffer contents, NVIDIA ioctl structures or potential BPF data.
RAW_SYSCALLS = (
    "ioctl,read,readv,write,writev,recvmsg,sendmsg,recvfrom,sendto,"
    "bpf,io_uring_setup"
)


def read_local(path):
    try:
        return Path(path).read_text().strip()
    except OSError as error:
        return f"unavailable: {error.strerror}"


def write_all(fd, data):
    remaining = memoryview(data)
    while remaining:
        remaining = remaining[os.write(fd, remaining):]


def synced_file(path, data):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_DSYNC, FILE_MODE)
    try:
        write_all(fd, data.encode())
    finally:
        os.close(fd)


def stop_group(process):
    if process.poll() is not None:
        return
    for sig in (signal.SIGINT, signal.SIGKILL):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            return
        try:
            process.wait(timeout=STOP_GRACE_SECONDS)
            return
        except subprocess.TimeoutExpired:
            pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/ssk")
    parser.add_argument("--strace", type=Path)
    args = parser.parse_args()
    if platform.system() != "Linux":
        parser.error("Linux-only; no native window will be started")
    tracer = args.strace or shutil.which("strace")
    if not tracer:
        tracer = ROOT / "runtime/diag-tools/unpacked/usr/bin/strace"
    tracer = Path(tracer).resolve()
    binary = args.binary.resolve()
    for path in (tracer, binary):
        if not path.is_file() or not os.access(path, os.X_OK):
            parser.error(f"executable missing: {path}; no probe started")
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    folder = ROOT / "runtime" / f"graphics-{stamp}-{os.getpid()}"
    folder.mkdir(mode=DIR_MODE, parents=True)
    command = [
        str(tracer), "-f", "-ttt", "-T", "-I", "1", "--kill-on-exit",
        "--decode-fds=path", "-e", f"trace={SYSCALLS}",
        "-e", f"raw={RAW_SYSCALLS}",
        str(binary), "graphics-probe",
        "--log", str(folder / "stages.log"),
    ]
    metadata = {
        "command": command,
        "kernel": platform.release(),
        "boot_id": read_local("/proc/sys/kernel/random/boot_id"),
        "nvidia": read_local("/proc/driver/nvidia/version"),
        "session": {name: os.environ.get(name) for name in (
            "XDG_SESSION_TYPE", "DISPLAY", "WAYLAND_DISPLAY",
            "WAYLAND_SOCKET", "WGPU_POWER_PREF",
            "VK_DRIVER_FILES", "VK_ICD_FILENAMES",
        )},
        "limits": {"seconds": DEFAULT_TIMEOUT_SECONDS, "trace_bytes": MAX_TRACE_BYTES},
        "note": "No SSK game/audio/network. ptrace; no --seccomp-bpf. Buffers/ioctls raw.",
    }
    synced_file(folder / "metadata.json", json.dumps(metadata, indent=2) + "\n")
    print(f"Graphics-only probe; {DEFAULT_TIMEOUT_SECONDS}s / {MAX_TRACE_BYTES // (1024 * 1024)}MiB cap.")
    print(f"Logs: {folder}", flush=True)
    fd = os.open(folder / "syscalls.log", os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_DSYNC, FILE_MODE)
    process = None
    reason = "completed"
    written = 0
    try:
        # strace diagnostics and child stderr share this pipe; stage markers also
        # have their own synced file. stdout is local and synchronous as well.
        process = subprocess.Popen(command, cwd=ROOT, stdout=fd, stderr=subprocess.PIPE,
                                   start_new_session=True, bufsize=0)
        deadline = time.monotonic() + DEFAULT_TIMEOUT_SECONDS
        with selectors.DefaultSelector() as selector:
            selector.register(process.stderr, selectors.EVENT_READ)
            while True:
                if time.monotonic() >= deadline:
                    reason = "time limit"
                    break
                if not selector.select(timeout=1):
                    if process.poll() is not None:
                        break
                    continue
                data = os.read(process.stderr.fileno(), READ_CHUNK_BYTES)
                if not data:
                    break
                available = MAX_TRACE_BYTES - written
                write_all(fd, data[:available])
                written += min(len(data), available)
                if written >= MAX_TRACE_BYTES:
                    reason = "size limit"
                    break
        if reason == "completed":
            try:
                process.wait(timeout=STOP_GRACE_SECONDS)
            except subprocess.TimeoutExpired:
                reason = "stderr closed before exit"
    except KeyboardInterrupt:
        reason = "interrupted"
    except OSError as error:
        reason = f"capture error: {error}"
    finally:
        if process is not None:
            stop_group(process)
            process.stderr.close()
        os.close(fd)
    result = {"reason": reason, "returncode": process.poll() if process else None,
              "trace_bytes": written}
    synced_file(folder / "result.json", json.dumps(result, indent=2) + "\n")
    print(f"{reason}; exit={result['returncode']}; logs kept locally.")
    return 0 if reason == "completed" and result["returncode"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
