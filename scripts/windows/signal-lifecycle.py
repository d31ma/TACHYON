from __future__ import annotations

import json
import os
import pathlib
import queue
import signal
import subprocess
import sys
import tempfile
import threading
from typing import TextIO


def start_line_reader(pipe: TextIO) -> queue.Queue[str]:
    output: queue.Queue[str] = queue.Queue()

    def read_all_lines() -> None:
        for line in pipe:
            output.put(line.rstrip("\r\n"))

    threading.Thread(target=read_all_lines, daemon=True).start()
    return output


def next_line(output: queue.Queue[str], description: str) -> str:
    try:
        return output.get(timeout=10)
    except queue.Empty as error:
        raise RuntimeError(f"timed out waiting for {description}") from error


def write_project(project: pathlib.Path) -> None:
    page = project / "client" / "pages" / "tac.html"
    page.parent.mkdir(parents=True)
    page.write_text("<signal-page />", encoding="utf-8")
    view = project / "client" / "components" / "signal" / "page" / "tac.html"
    view.parent.mkdir(parents=True)
    view.write_text('<main aria-label="Signal"><h1>Signal</h1></main>', encoding="utf-8")


def start_server(binary: pathlib.Path, project: pathlib.Path) -> subprocess.Popen[str]:
    return subprocess.Popen(
        [
            str(binary),
            "serve",
            str(project),
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            "--no-watch",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
    )


def check_startup(stdout: queue.Queue[str], stderr: queue.Queue[str]) -> None:
    startup = json.loads(next_line(stderr, "handler startup"))
    if startup.get("installed") != ["CTRL_C", "CTRL_BREAK"]:
        raise RuntimeError(f"unexpected startup event: {startup}")
    ready = next_line(stdout, "server readiness")
    if "Tachyon server ready" not in ready:
        raise RuntimeError(f"unexpected readiness line: {ready}")


def check_break_signal(child: subprocess.Popen[str]) -> None:
    if child.stdout is None or child.stderr is None:
        raise RuntimeError("server pipes were not created")
    stdout = start_line_reader(child.stdout)
    stderr = start_line_reader(child.stderr)
    check_startup(stdout, stderr)

    os.kill(child.pid, signal.CTRL_BREAK_EVENT)
    receipt = json.loads(next_line(stderr, "CTRL-BREAK receipt"))
    if receipt.get("signal") != "CTRL_BREAK":
        raise RuntimeError(f"unexpected receipt event: {receipt}")
    status = child.wait(timeout=10)
    if status != 0:
        raise RuntimeError(f"server exited with status {status}")


def main() -> None:
    if len(sys.argv) != 2:
        raise RuntimeError("usage: signal-lifecycle.py <ty.exe>")
    binary = pathlib.Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="tachyon-signal-") as temporary:
        project = pathlib.Path(temporary)
        write_project(project)
        child = start_server(binary, project)
        try:
            check_break_signal(child)
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=5)


if __name__ == "__main__":
    main()
