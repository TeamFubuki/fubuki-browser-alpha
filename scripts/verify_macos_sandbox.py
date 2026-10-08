#!/usr/bin/env python3
"""Check live CEF child processes with the macOS Seatbelt API."""
import argparse
import ctypes
import os
import shlex
import signal
import subprocess
import time
from pathlib import Path


def processes():
    output = subprocess.check_output(["ps", "-axo", "pid=,ppid=,command="], text=True)
    return [(int(pid), int(parent), command) for line in output.splitlines()
            for pid, parent, command in [line.strip().split(None, 2)]]


def inspect(pid, check):
    rows = processes()
    descendants = {pid}
    while True:
        expanded = descendants | {child for child, parent, _ in rows if parent in descendants}
        if expanded == descendants:
            break
        descendants = expanded
    kinds = set()
    for child, _, command in rows:
        if child == pid or child not in descendants:
            continue
        switches = shlex.split(command)
        process_type = next((s.split("=", 1)[1] for s in switches if s.startswith("--type=")), "")
        if not process_type:
            continue
        if "--no-sandbox" in switches or "--disable-gpu-sandbox" in switches:
            raise RuntimeError(f"PID {child}: sandbox disabled by command line")
        state = check(child, None, 0)
        if state not in (0, 1):
            raise RuntimeError(f"PID {child}: sandbox_check failed ({state})")
        # Chromium explicitly exempts some utility services. Report them, but
        # never exempt renderers or GPU, and fail any sandboxed utility type.
        exempt = process_type == "utility" and "--service-sandbox-type=none" in switches
        if state != 1 and not exempt:
            raise RuntimeError(f"PID {child} ({process_type}) is not sandboxed")
        print(f"PID {child}: {process_type}, seatbelt={state}, exempt={exempt}", flush=True)
        kinds.add(process_type)
    return {"renderer", "gpu-process"}.issubset(kinds)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--pid", type=int, help="inspect an already running browser PID")
    group.add_argument("--launch", type=Path, help="launch and stop a test app (uses its normal profile)")
    parser.add_argument("--timeout", type=float, default=45)
    parser.add_argument("--attempt-disable", action="store_true", help="test that external disabling switches are ignored")
    args = parser.parse_args()
    if args.pid is not None and args.pid <= 0:
        parser.error("--pid must identify a running browser process")
    if args.attempt_disable and not args.launch:
        parser.error("--attempt-disable requires --launch")
    check = ctypes.CDLL("/usr/lib/libsandbox.1.dylib").sandbox_check
    check.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int]
    check.restype = ctypes.c_int
    child = None
    try:
        pid = args.pid
        if args.launch:
            executable = args.launch.resolve() / "Contents/MacOS/Fubuki Browser Alpha"
            for flag in ("--type=renderer", "-type=gpu-process"):
                result = subprocess.run([str(executable), flag], timeout=5)
                if result.returncode != 1:
                    raise RuntimeError(f"Main did not reject child-process entry: {flag}")
            flags = ["--no-sandbox", "--disable-gpu-sandbox"] if args.attempt_disable else []
            child = subprocess.Popen([str(executable), *flags], start_new_session=True)
            pid = child.pid
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            if child and child.poll() is not None:
                raise RuntimeError(f"browser exited early: {child.returncode}")
            if inspect(pid, check):
                # Check again after initialization to catch crashes/restarts.
                time.sleep(2)
                if (not child or child.poll() is None) and inspect(pid, check):
                    return
            time.sleep(1)
        raise RuntimeError("timed out waiting for sandboxed renderer and GPU processes")
    finally:
        if child:
            # Only terminate the process group created by this invocation.
            try:
                os.killpg(child.pid, signal.SIGTERM)
                child.wait(timeout=5)
            except ProcessLookupError:
                pass
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()


if __name__ == "__main__":
    main()
