#!/usr/bin/env python3

import importlib.util
import json
import os
import sys
import time
from pathlib import Path

from gi.repository import Gio, GLib


class WebProcessTree:
    ACCESSIBLE = "org.a11y.atspi.Accessible"
    TEXT = "org.a11y.atspi.Text"
    ACTION = "org.a11y.atspi.Action"
    ROOTS = "/org/a11y/webkit/accessible"
    MAX_NODES = 4096
    MAX_TEXT_LENGTH = 512
    POLL_SECONDS = 0.2

    def __init__(self):
        session = Gio.bus_get_sync(Gio.BusType.SESSION)
        address = session.call_sync("org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus", "GetAddress",
                                    None, GLib.VariantType("(s)"), Gio.DBusCallFlags.NONE, 5000, None).unpack()[0]
        flags = Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION
        self.bus = Gio.DBusConnection.new_for_address_sync(address, flags, None, None)

    def call(self, name, path, interface, method, arguments=None):
        return self.bus.call_sync(name, path, interface, method, arguments, None,
                                  Gio.DBusCallFlags.NONE, 5000, None).unpack()

    def web_process(self):
        names = self.call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames")[0]
        return next((name for name in names if ".Sandboxed.WebProcess-" in name or ".WebProcess-" in name), None)

    def roots(self, name):
        node = self.call(name, self.ROOTS, "org.freedesktop.DBus.Introspectable", "Introspect")[0]
        return [(name, f"{self.ROOTS}/{child.path}") for child in Gio.DBusNodeInfo.new_for_xml(node).nodes]

    def nodes(self):
        name = self.web_process()
        pending = self.roots(name) if name is not None else []
        seen = []
        while pending and len(seen) < self.MAX_NODES:
            node = pending.pop()
            seen.append(node)
            pending.extend(self.call(node[0], node[1], self.ACCESSIBLE, "GetChildren")[0])
        return seen

    def texts(self, node):
        name = self.call(node[0], node[1], "org.freedesktop.DBus.Properties", "Get",
                         GLib.Variant("(ss)", (self.ACCESSIBLE, "Name")))[0]
        try:
            text = self.call(node[0], node[1], self.TEXT, "GetText", GLib.Variant("(ii)", (0, self.MAX_TEXT_LENGTH)))[0]
        except GLib.Error:
            text = ""
        return {name, text}

    def exposed(self):
        found = set()
        for node in self.nodes():
            found |= self.texts(node)
        return found

    def wait_for_controls(self, required, seconds):
        deadline = time.monotonic() + seconds
        exposed = set()
        while time.monotonic() < deadline:
            exposed = self.exposed()
            if set(required) <= exposed:
                print("OK:", ", ".join(required), flush=True)
                return
            time.sleep(self.POLL_SECONDS)
        raise AssertionError(f"Missing {set(required) - exposed} in the web process tree; exposed: {sorted(exposed)}")

    def activate_control(self, name):
        self.wait_for_controls([name], YonOriginProbe.WAIT_SECONDS)
        for node in self.nodes():
            if name in self.texts(node) and self.call(node[0], node[1], self.ACTION, "DoAction", GLib.Variant("(i)", (0,)))[0]:
                return
        raise AssertionError(f"Rendered control {name!r} has no accessible activation action")


class YonOriginProbe:
    WAIT_SECONDS = 30

    @staticmethod
    def load_accessibility_probe():
        path = Path(__file__).with_name("a11y-probe.py")
        specification = importlib.util.spec_from_file_location("accessibility_probe", path)
        module = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(module)
        return module

    @staticmethod
    def window_tree(probe):
        return {
            "wait": lambda texts: probe.wait_for_controls(texts, YonOriginProbe.WAIT_SECONDS),
            "click": probe.activate_control,
        }

    @staticmethod
    def web_process_tree(probe):
        tree = WebProcessTree()
        print("Reading the page from the web process's own accessibility tree, because GTK does not "
              "embed it in the window under Wayland", flush=True)
        return {
            "wait": lambda texts: tree.wait_for_controls(texts, YonOriginProbe.WAIT_SECONDS),
            "click": tree.activate_control,
        }

    @staticmethod
    def main(steps):
        probe = YonOriginProbe.load_accessibility_probe()
        probe.Atspi.init()
        probe.wait_for_application()
        trees = {"wayland": YonOriginProbe.web_process_tree, "x11": YonOriginProbe.window_tree}
        actions = trees[os.environ.get("GDK_BACKEND", "x11")](probe)
        for kind, value in steps:
            actions[kind](value)
        print("PASS: the Linux app showed every Yon answer", flush=True)


if __name__ == "__main__":
    YonOriginProbe.main(json.loads(sys.argv[1]))
