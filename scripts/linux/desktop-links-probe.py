#!/usr/bin/env python3

import importlib.util
import json
import sys
import time
from pathlib import Path


class DesktopLinksProbe:
    WAIT_SECONDS = 60

    @staticmethod
    def load_accessibility_probe():
        path = Path(__file__).with_name("a11y-probe.py")
        specification = importlib.util.spec_from_file_location("accessibility_probe", path)
        module = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(module)
        return module

    @staticmethod
    def wait_until_gone(probe, texts):
        deadline = time.monotonic() + DesktopLinksProbe.WAIT_SECONDS
        shown = set()
        while time.monotonic() < deadline:
            root = probe.find_application()
            shown = set(texts) & probe.exposed_texts(root) if root is not None else set()
            if root is not None and not shown:
                print("OK gone:", ", ".join(texts), flush=True)
                return
            time.sleep(probe.ProbeLimits.POLL_SECONDS)
        raise AssertionError(f"the page still shows {sorted(shown)}")

    @staticmethod
    def run_step(probe, kind, value):
        if kind == "wait":
            probe.wait_for_controls(value, DesktopLinksProbe.WAIT_SECONDS)
        elif kind == "gone":
            DesktopLinksProbe.wait_until_gone(probe, value)
        elif kind == "click":
            probe.activate_control(value)
        else:
            raise AssertionError(f"unknown probe step {kind!r}")

    @staticmethod
    def main(steps):
        probe = DesktopLinksProbe.load_accessibility_probe()
        probe.Atspi.init()
        probe.wait_for_application()
        for kind, value in steps:
            DesktopLinksProbe.run_step(probe, kind, value)


if __name__ == "__main__":
    DesktopLinksProbe.main(json.loads(sys.argv[1]))
