from __future__ import annotations

import json
import os
import pathlib
import queue
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
from typing import TextIO

READY = "Tachyon preview ready"
RESTARTING = "Restarting the preview"
WAIT_SECONDS = 90
STOP_SECONDS = 30


def start_line_reader(pipe: TextIO) -> queue.Queue[str]:
    output: queue.Queue[str] = queue.Queue()

    def read_all_lines() -> None:
        for line in pipe:
            output.put(line.rstrip("\r\n"))

    threading.Thread(target=read_all_lines, daemon=True).start()
    return output


def line_containing(output: queue.Queue[str], expected: str) -> str:
    deadline = time.monotonic() + WAIT_SECONDS
    seen: list[str] = []
    while time.monotonic() < deadline:
        try:
            line = output.get(timeout=1)
        except queue.Empty:
            continue
        seen.append(line)
        if expected in line:
            return line
    raise RuntimeError(f"no line containing {expected!r} within {WAIT_SECONDS} s; saw {seen[-20:]}")


def write_project(project: pathlib.Path) -> None:
    page = project / "client" / "pages" / "tac.html"
    page.parent.mkdir(parents=True)
    page.write_text("<restart-page />", encoding="utf-8")
    view = project / "client" / "components" / "restart" / "page" / "tac.html"
    view.parent.mkdir(parents=True)
    view.write_text('<main aria-label="Restart"><h1>Restart</h1></main>', encoding="utf-8")


def free_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def page_status(port: int) -> int:
    with urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=10) as response:
        return response.status


def processes(where: str) -> list[dict[str, object]]:
    query = (
        f"@(Get-CimInstance Win32_Process -Filter '{where}' | "
        "Select-Object ProcessId, ParentProcessId, ExecutablePath) | ConvertTo-Json -Compress"
    )
    shown = subprocess.run(
        ["powershell", "-NoProfile", "-Command", query], capture_output=True, text=True, check=True
    ).stdout.strip()
    found = json.loads(shown) if shown else []
    return found if isinstance(found, list) else [found]


def replace_running(executable: pathlib.Path, contents: bytes) -> None:
    try:
        executable.write_bytes(contents)
        print("NOTE: Windows let the running ty.exe be overwritten in place", flush=True)
    except PermissionError as error:
        print(f"NOTE: Windows refused to overwrite the running ty.exe ({error}); it was renamed aside first", flush=True)
        executable.rename(executable.with_name("ty-previous.exe"))
        executable.write_bytes(contents)


def restarted_child(parent: int, executable: pathlib.Path) -> int:
    for entry in processes(f"ParentProcessId = {parent}"):
        if str(entry.get("ExecutablePath") or "").lower() == str(executable).lower():
            return int(entry["ProcessId"])
    raise RuntimeError(f"the first ty ({parent}) started no new {executable} as its child")


def check_restart(preview: subprocess.Popen[str], executable: pathlib.Path, port: int) -> None:
    if preview.stdout is None:
        raise RuntimeError("the preview stdout pipe was not created")
    stdout = start_line_reader(preview.stdout)
    line_containing(stdout, READY)
    if page_status(port) != 200:
        raise RuntimeError("the first preview did not answer its page")
    print(f"OK: the first ty ({preview.pid}) answered its page on port {port}", flush=True)

    replace_running(executable, executable.with_name("ty-next.exe").read_bytes())
    line_containing(stdout, RESTARTING)
    line_containing(stdout, READY)
    if preview.poll() is not None:
        raise RuntimeError(f"the first ty exited with {preview.returncode} instead of waiting for the new one")
    child = restarted_child(preview.pid, executable)
    if page_status(port) != 200:
        raise RuntimeError("the restarted preview did not answer its page")
    print(f"OK: the new ty.exe ({child}) runs as a child of the first ty ({preview.pid}) and answers on port {port}", flush=True)

    os.kill(preview.pid, signal.CTRL_BREAK_EVENT)
    status = preview.wait(timeout=STOP_SECONDS)
    left = processes(f"ProcessId = {child}")
    if left:
        raise RuntimeError(f"the first ty exited with {status} while the new ty ({child}) still ran")
    if status != 0:
        raise RuntimeError(f"CTRL-BREAK stopped the new ty, but the first ty exited with {status}, not with the new one's code 0")
    print("OK: CTRL-BREAK stopped the new ty, and the first ty then exited with its code 0", flush=True)


def main() -> None:
    if len(sys.argv) != 2:
        raise RuntimeError("usage: self-restart.py <ty.exe>")
    binary = pathlib.Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="tachyon-self-restart-") as temporary:
        root = pathlib.Path(temporary)
        tools = root / "tools"
        tools.mkdir()
        executable = tools / "ty.exe"
        shutil.copyfile(binary, executable)
        (tools / "ty-next.exe").write_bytes(binary.read_bytes() + b"\0tachyon self-restart\n")
        project = root / "project"
        write_project(project)
        port = free_port()
        preview = subprocess.Popen(
            [str(executable), "preview", str(project), "--host", "127.0.0.1", "--port", str(port)],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
        )
        try:
            check_restart(preview, executable, port)
        finally:
            for entry in processes(f"ParentProcessId = {preview.pid}"):
                subprocess.run(["taskkill", "/F", "/T", "/PID", str(entry["ProcessId"])], capture_output=True)
            if preview.poll() is None:
                preview.kill()
                preview.wait(timeout=10)
    print("PASS: ty preview restarted itself on Windows as a child process and exited with that child's code", flush=True)


if __name__ == "__main__":
    main()
