#!/usr/bin/env python3

import importlib.util
import json
import subprocess
import sys
import threading
import time
from pathlib import Path


class PageStepsProbe:
    WAIT_SECONDS = 120
    SIZE_TOLERANCE = 2
    MENU_BAR_ROOM = 64
    DIALOG_SECONDS = 4
    CLICK_SECONDS = 45

    @staticmethod
    def load_accessibility_probe():
        path = Path(__file__).with_name("a11y-probe.py")
        specification = importlib.util.spec_from_file_location("accessibility_probe", path)
        module = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(module)
        return module

    @staticmethod
    def frame(probe):
        root = probe.find_application()
        for node in probe.walk_tree(root):
            if node.get_role() == probe.Atspi.Role.FRAME:
                return node
        raise AssertionError("the application shows no window")

    @staticmethod
    def wait_for_size(probe, size):
        deadline = time.monotonic() + PageStepsProbe.WAIT_SECONDS
        extents = None
        while time.monotonic() < deadline:
            extents = PageStepsProbe.frame(probe).get_extents(probe.Atspi.CoordType.SCREEN)
            width_matches = abs(extents.width - size[0]) <= PageStepsProbe.SIZE_TOLERANCE
            height_matches = size[1] - PageStepsProbe.SIZE_TOLERANCE <= extents.height <= size[1] + PageStepsProbe.MENU_BAR_ROOM
            if width_matches and height_matches:
                print(f"OK: window {extents.width}x{extents.height}", flush=True)
                return
            time.sleep(probe.ProbeLimits.POLL_SECONDS)
        raise AssertionError(f"the window is {extents.width}x{extents.height}, not {size[0]}x{size[1]} plus its menu bar")

    @staticmethod
    def click_in_background(probe, control):
        def click():
            try:
                probe.activate_control(control)
            except Exception as problem:
                print(f"NOTE: the click on {control!r} answered late: {problem}", flush=True)
        worker = threading.Thread(target=click, daemon=True)
        worker.start()
        return worker

    @staticmethod
    def press(keys):
        for key in keys:
            command = ["xdotool", "type", "--delay", "20", key["type"]] if isinstance(key, dict) else ["xdotool", "key", key]
            subprocess.run(command, check=True)
            time.sleep(0.3)

    @staticmethod
    def pick(probe, choice):
        worker = PageStepsProbe.click_in_background(probe, choice["control"])
        time.sleep(PageStepsProbe.DIALOG_SECONDS)
        PageStepsProbe.press(choice["keys"])
        worker.join(PageStepsProbe.CLICK_SECONDS)
        print(f"OK: the file dialog took {choice['keys']}", flush=True)

    @staticmethod
    def main(steps):
        probe = PageStepsProbe.load_accessibility_probe()
        probe.Atspi.init()
        started = time.monotonic()
        probe.wait_for_application()
        actions = {
            "wait": lambda texts: probe.wait_for_controls(texts, PageStepsProbe.WAIT_SECONDS),
            "click": probe.activate_control,
            "size": lambda size: PageStepsProbe.wait_for_size(probe, size),
            "pick": lambda choice: PageStepsProbe.pick(probe, choice),
        }
        for index, (kind, value) in enumerate(steps):
            actions[kind](value)
            if index == 0:
                print(f"LOADED: the first page answered {time.monotonic() - started:.1f} s after launch", flush=True)
        print("PASS: the Linux app showed every page answer", flush=True)


if __name__ == "__main__":
    PageStepsProbe.main(json.loads(sys.argv[1]))
