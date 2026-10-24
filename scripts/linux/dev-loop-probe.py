#!/usr/bin/env python3

import importlib.util
import json
import os
import sys
import time
from pathlib import Path


class DevLoopProbe:
    WAIT_SECONDS = 60
    RETRY_SECONDS = 3
    POLL_SECONDS = 0.25

    def __init__(self, project, log):
        self.project = Path(project)
        self.log = Path(log)
        path = Path(__file__).with_name("a11y-probe.py")
        specification = importlib.util.spec_from_file_location("accessibility_probe", path)
        self.probe = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(self.probe)

    def wait(self, texts):
        self.probe.wait_for_controls(texts, self.WAIT_SECONDS)

    def click(self, name):
        self.probe.activate_control(name)

    def click_until(self, step):
        name, text = step
        deadline = time.monotonic() + self.WAIT_SECONDS
        while True:
            self.click(name)
            try:
                self.probe.wait_for_controls([text], self.RETRY_SECONDS)
                return
            except AssertionError:
                if time.monotonic() > deadline:
                    raise

    def edit(self, step):
        relative, old, new = step
        path = self.project / relative
        source = path.read_text()
        assert old in source, f"{relative} does not hold {old!r}"
        path.write_text(source.replace(old, new))
        print(f"EDIT: {relative}: {old!r} -> {new!r}", flush=True)

    def opened(self, count):
        deadline = time.monotonic() + self.WAIT_SECONDS * 4
        while time.monotonic() < deadline:
            seen = self.log.read_text(errors="replace").count("Opened the linux app")
            if seen >= count:
                print(f"OK: ty preview opened the app {seen} time(s)", flush=True)
                return
            time.sleep(self.POLL_SECONDS)
        raise AssertionError(f"ty preview did not open the app {count} time(s):\n{self.log.read_text(errors='replace')}")

    def main(self, steps):
        self.opened(1)
        self.probe.Atspi.init()
        self.probe.wait_for_application()
        actions = {
            "wait": self.wait,
            "click": self.click,
            "clickUntil": self.click_until,
            "edit": self.edit,
            "opened": self.opened,
        }
        for kind, value in steps:
            actions[kind](value)
        print("PASS: every edit reached the Linux app", flush=True)


if __name__ == "__main__":
    DevLoopProbe(os.environ["DEV_LOOP_PROJECT"], os.environ["DEV_LOOP_LOG"]).main(json.loads(sys.argv[1]))
