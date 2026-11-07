#!/usr/bin/env python3

import importlib.util
import json
import os
import sys
import time
from pathlib import Path


class PageChecksProbe:
    WAIT_SECONDS = 30
    REQUEST_SECONDS = 60
    POLL_SECONDS = 0.1
    KEY_NAMES = {"Left": 0xFF51, "F10": 0xFFC7}
    probe = None

    @staticmethod
    def load_accessibility_probe():
        path = Path(__file__).with_name("a11y-probe.py")
        specification = importlib.util.spec_from_file_location("accessibility_probe", path)
        module = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(module)
        return module

    @staticmethod
    def find_entry(name):
        root = PageChecksProbe.probe.find_application()
        for node in PageChecksProbe.probe.walk_tree(root):
            if node.get_role() == PageChecksProbe.probe.Atspi.Role.ENTRY and node.get_name() == name:
                return node
        raise AssertionError(f"No text entry named {name!r}")

    @staticmethod
    def entry_state(name):
        atspi = PageChecksProbe.probe.Atspi
        entry = PageChecksProbe.find_entry(name)
        text = entry.get_text_iface()
        value = atspi.Text.get_text(text, 0, atspi.Text.get_character_count(text))
        focused = entry.get_state_set().contains(atspi.StateType.FOCUSED)
        return {"text": value, "caret": atspi.Text.get_caret_offset(text), "focused": focused}

    @staticmethod
    def wait_for_entry(name, expected, shown):
        deadline = time.monotonic() + PageChecksProbe.WAIT_SECONDS
        state = {}
        exposed = set()
        while time.monotonic() < deadline:
            state = PageChecksProbe.entry_state(name)
            exposed = PageChecksProbe.probe.exposed_texts(PageChecksProbe.probe.find_application())
            if all(state.get(key) == value for key, value in expected.items()) and (shown is None or shown in exposed):
                print(f"OK: {name} holds {state['text']!r} with the caret at {state['caret']} and focus; page shows {shown or 'the same text'!r}", flush=True)
                return
            time.sleep(PageChecksProbe.POLL_SECONDS)
        page = sorted(text for text in exposed if text and text.startswith("Typed"))
        raise AssertionError(f"{name}: wanted {expected} and {shown!r}; the entry has {state}; the page shows {page}")

    @staticmethod
    def press(key):
        atspi = PageChecksProbe.probe.Atspi
        symbol = PageChecksProbe.KEY_NAMES.get(key, ord(key[0]))
        atspi.generate_keyboard_event(symbol, None, atspi.KeySynthType.SYM)

    @staticmethod
    def focus(name):
        atspi = PageChecksProbe.probe.Atspi
        PageChecksProbe.probe.wait_for_controls([name])
        entry = PageChecksProbe.find_entry(name)
        atspi.Component.grab_focus(entry.get_component_iface())
        deadline = time.monotonic() + PageChecksProbe.WAIT_SECONDS
        while time.monotonic() < deadline:
            if PageChecksProbe.entry_state(name)["focused"]:
                print(f"OK: {name} has focus", flush=True)
                return
            time.sleep(PageChecksProbe.POLL_SECONDS)
        raise AssertionError(f"{name} did not take focus")

    @staticmethod
    def type_keys(step):
        name = step["entry"]
        state = PageChecksProbe.entry_state(name)
        text = state["text"]
        caret = state["caret"]
        for key in step["keys"]:
            PageChecksProbe.press(key)
            text = text[:caret] + key + text[caret:]
            caret += 1
            if step["pace"] == "each":
                PageChecksProbe.wait_for_entry(name, {"text": text, "caret": caret, "focused": True}, step["shown"] + text)
        if step["pace"] == "burst":
            PageChecksProbe.wait_for_entry(name, {"text": text, "caret": caret, "focused": True}, step["shown"] + text)

    @staticmethod
    def move_caret(step):
        for key in step["keys"]:
            PageChecksProbe.press(key)
        PageChecksProbe.wait_for_entry(step["entry"], {"caret": step["caret"], "focused": True}, None)

    @staticmethod
    def ask(name):
        folder = Path(os.environ["TAC_PROBE_HANDSHAKE"])
        (folder / f"{name}.asked").write_text("asked\n")
        deadline = time.monotonic() + PageChecksProbe.REQUEST_SECONDS
        while time.monotonic() < deadline:
            if (folder / f"{name}.done").exists():
                print(f"OK: {name} done", flush=True)
                return
            time.sleep(PageChecksProbe.POLL_SECONDS)
        raise AssertionError(f"Nobody answered the request {name!r} in {PageChecksProbe.REQUEST_SECONDS} seconds")

    @staticmethod
    def quit(name):
        atspi = PageChecksProbe.probe.Atspi
        PageChecksProbe.press("F10")
        PageChecksProbe.probe.wait_for_roles([(atspi.Role.MENU_ITEM, name)])
        PageChecksProbe.probe.activate_control(name)
        deadline = time.monotonic() + PageChecksProbe.WAIT_SECONDS
        while time.monotonic() < deadline:
            if PageChecksProbe.probe.find_application() is None:
                print("OK: the app left the accessibility bus", flush=True)
                return
            time.sleep(PageChecksProbe.POLL_SECONDS)
        raise AssertionError(f"The app stayed on the accessibility bus after {name}")

    @staticmethod
    def main(steps):
        PageChecksProbe.probe = PageChecksProbe.load_accessibility_probe()
        PageChecksProbe.probe.Atspi.init()
        PageChecksProbe.probe.wait_for_application()
        actions = {
            "wait": lambda texts: PageChecksProbe.probe.wait_for_controls(texts, PageChecksProbe.WAIT_SECONDS),
            "click": PageChecksProbe.probe.activate_control,
            "focus": PageChecksProbe.focus,
            "type": PageChecksProbe.type_keys,
            "move": PageChecksProbe.move_caret,
            "ask": PageChecksProbe.ask,
            "quit": PageChecksProbe.quit,
        }
        for kind, value in steps:
            actions[kind](value)
        print("PASS: the Linux app showed every page check", flush=True)


if __name__ == "__main__":
    PageChecksProbe.main(json.loads(sys.argv[1]))
