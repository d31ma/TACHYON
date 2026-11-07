from __future__ import annotations

import signal
import subprocess
import sys


def start_group(command: list[str]) -> subprocess.Popen[bytes]:
    return subprocess.Popen(
        command,
        stdin=subprocess.DEVNULL,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
    )


def wait_for_break(child: subprocess.Popen[bytes]) -> None:
    for line in sys.stdin:
        if line.strip() == "break":
            child.send_signal(signal.CTRL_BREAK_EVENT)
            return
    child.kill()


def main() -> None:
    if len(sys.argv) < 2:
        raise RuntimeError("usage: process-group.py <program> [arguments...]")
    child = start_group(sys.argv[1:])
    try:
        wait_for_break(child)
        sys.exit(child.wait())
    finally:
        if child.poll() is None:
            child.kill()
            child.wait(timeout=10)


if __name__ == "__main__":
    main()
