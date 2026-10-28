#!/usr/bin/env python3

import importlib.util
import json
import subprocess
import sys
import threading
import time
from pathlib import Path

from gi.repository import GLib


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
    def focus_portal_dialog():
        for search in (["--name", "Pick a File"], ["--class", "xdg-desktop-portal-gtk"]):
            found = subprocess.run(["xdotool", "search", "--onlyvisible", *search], capture_output=True, text=True)
            windows = found.stdout.split()
            if windows:
                subprocess.run(["xdotool", "windowactivate", "--sync", windows[-1]], check=True)
                print(f"OK: focused the desktop portal's file dialog ({' '.join(search)})", flush=True)
                return

    @staticmethod
    def pick(probe, choice):
        worker = PageStepsProbe.click_in_background(probe, choice["control"])
        time.sleep(PageStepsProbe.DIALOG_SECONDS)
        PageStepsProbe.focus_portal_dialog()
        PageStepsProbe.press(choice["keys"])
        worker.join(PageStepsProbe.CLICK_SECONDS)
        print(f"OK: the file dialog took {choice['keys']}", flush=True)

    @staticmethod
    def pointer_click(probe, name):
        root = probe.wait_for_controls([name], PageStepsProbe.WAIT_SECONDS)
        for node in probe.walk_tree(root):
            extents = node.get_extents(probe.Atspi.CoordType.SCREEN) if node.get_name() == name else None
            if extents is not None and extents.width > 0 and extents.height > 0:
                x, y = extents.x + extents.width // 2, extents.y + extents.height // 2
                subprocess.run(["xdotool", "mousemove", str(x), str(y), "click", "1"], check=True)
                print(f"OK: a pointer click at {x},{y} on {name!r}", flush=True)
                return
        raise AssertionError(f"Rendered control {name!r} has no on-screen extents")

    @staticmethod
    def current_texts(probe):
        try:
            root = probe.find_application()
            return probe.exposed_texts(root) if root is not None else set()
        except GLib.Error as error:
            print(f"OK: a page node changed while it was read ({error.message}); reading again", flush=True)
            return set()

    @staticmethod
    def verdict(probe, wanted):
        deadline = time.monotonic() + wanted["seconds"]
        texts = set()
        while time.monotonic() < deadline and not texts & {wanted["passed"], wanted["failed"]}:
            texts = PageStepsProbe.current_texts(probe)
            time.sleep(probe.ProbeLimits.POLL_SECONDS)
        for text in sorted(item for item in texts if item and item.startswith(tuple(wanted["lines"]))):
            print(f"REPORT: {text}", flush=True)
        assert wanted["passed"] in texts, f"the page did not show {wanted['passed']!r}; exposed: {sorted(texts)}"
        print(f"OK: {wanted['passed']}", flush=True)

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
            "verdict": lambda wanted: PageStepsProbe.verdict(probe, wanted),
            "pointer": lambda name: PageStepsProbe.pointer_click(probe, name),
        }
        for index, (kind, value) in enumerate(steps):
            actions[kind](value)
            if index == 0:
                print(f"LOADED: the first page answered {time.monotonic() - started:.1f} s after launch", flush=True)
        print("PASS: the Linux app showed every page answer", flush=True)


if __name__ == "__main__":
    PageStepsProbe.main(json.loads(sys.argv[1]))
