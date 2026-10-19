#!/usr/bin/env python3
"""Inspect or explicitly arm temporary kernel-lockup capture; never trigger a crash."""
import argparse
import json
import os
from pathlib import Path
import stat

STATE = Path("/run/ssk-freeze-watchdog.json")
BOOT_ID = Path("/proc/sys/kernel/random/boot_id")
KNOBS = {
    "hardlockup_panic": 1,
    "softlockup_panic": 1,
    "hardlockup_all_cpu_backtrace": 1,
    "softlockup_all_cpu_backtrace": 1,
    "panic": 60,
}
FILE_MODE = 0o600


def knob(name):
    return Path("/proc/sys/kernel") / name


def values():
    return {name: int(knob(name).read_text()) for name in KNOBS}


def apply(settings):
    for name, value in settings.items():
        knob(name).write_text(f"{value}\n")
        if int(knob(name).read_text()) != value:
            raise RuntimeError(f"sysctl verification failed: {name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("status", "arm", "restore"))
    args = parser.parse_args()
    if args.action == "status":
        print(json.dumps({
            "settings": values(),
            "kdump_loaded": Path("/sys/kernel/kexec_crash_loaded").read_text().strip(),
            "watchdog": knob("watchdog").read_text().strip(),
            "nmi_watchdog": knob("nmi_watchdog").read_text().strip(),
            "saved_state_exists": STATE.exists(),
        }, indent=2))
        return
    if os.geteuid() != 0:
        parser.error("arm/restore require graphical Polkit: pkexec /usr/bin/python3 /absolute/script.py ACTION")
    if args.action == "restore":
        info = STATE.lstat()
        if info.st_uid != 0 or not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != FILE_MODE:
            raise RuntimeError("Saved state must be a regular root-owned mode-0600 file")
        saved = json.loads(STATE.read_text())
        if saved["boot_id"] != BOOT_ID.read_text().strip():
            raise RuntimeError("Saved state belongs to another boot")
        previous = saved["previous"]
        if set(previous) != set(KNOBS) or not all(type(v) is int and v >= 0 for v in previous.values()):
            raise RuntimeError("Invalid saved settings")
        apply(previous)
        STATE.unlink()
        print("Previous sysctls restored. No persistent OS configuration changed.")
        return
    if Path("/sys/kernel/kexec_crash_loaded").read_text().strip() != "1":
        raise RuntimeError("No crash kernel loaded; refusing to arm")
    if any(int(knob(name).read_text()) != 1 for name in ("watchdog", "nmi_watchdog")):
        raise RuntimeError("Existing watchdog/NMI must already be enabled; refusing to reconfigure them")
    previous = values()
    saved = {"boot_id": BOOT_ID.read_text().strip(), "previous": previous, "armed": KNOBS}
    fd = os.open(STATE, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, FILE_MODE)
    try:
        with os.fdopen(fd, "w") as output:
            json.dump(saved, output, indent=2)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        try:
            apply(KNOBS)
        except BaseException:
            apply(previous)
            STATE.unlink()
            raise
    except BaseException:
        print(f"Arming failed; inspect {STATE} and current sysctls before retrying.")
        raise
    print("Armed for this boot: detected soft/hard lockup => panic => existing kdump.")
    print("Fallback panic reboot: 60 seconds. No crash, game or probe was started.")
    print("Restore with this script's restore action or reboot; nothing written to /etc.")


if __name__ == "__main__":
    main()
