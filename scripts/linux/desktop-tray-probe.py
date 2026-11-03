#!/usr/bin/env python3

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib


class DesktopTrayProbe:
    WAIT_SECONDS = 60
    POLL_SECONDS = 0.25
    WATCHER = "org.kde.StatusNotifierWatcher"
    ITEM = "org.kde.StatusNotifierItem"
    ITEM_PATH = "/StatusNotifierItem"
    MENU_PATH = "/StatusNotifierItem/Menu"
    MENU = "com.canonical.dbusmenu"
    PROPERTIES = "org.freedesktop.DBus.Properties"
    REMOTE_DESKTOP = "org.gnome.Mutter.RemoteDesktop"
    ALT = 0xFFE9
    F4 = 0xFFC1
    SHIFT = 0xFFE1

    def __init__(self, pid, log, screenshots):
        self.pid = pid
        self.log = Path(log)
        self.screenshots = screenshots
        self.service = f"{self.ITEM}-{pid}-1"
        self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        self.on_x11 = os.environ.get("XDG_SESSION_TYPE") == "x11"

    def call(self, name, path, interface, method, parameters=None):
        answer = self.bus.call_sync(name, path, interface, method, parameters, None, Gio.DBusCallFlags.NONE, 5000, None)
        return answer.unpack() if answer is not None else ()

    def running(self):
        try:
            state = Path(f"/proc/{self.pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
        except (FileNotFoundError, IndexError):
            return False
        return state != "Z"

    def wait(self, check, what):
        deadline = time.monotonic() + self.WAIT_SECONDS
        while time.monotonic() < deadline:
            if check():
                return
            time.sleep(self.POLL_SECONDS)
        raise AssertionError(f"waited {self.WAIT_SECONDS} s for {what}")

    def events(self):
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines() if line.strip()]

    def recorded(self, event, route=None):
        return any(entry["event"] == event and route in (None, entry["route"]) for entry in self.events())

    def screenshot(self, name):
        if not self.screenshots:
            return
        session = Path(__file__).with_name("linux-display-session.mjs")
        subprocess.run(["node", str(session), "screenshot", str(Path(self.screenshots) / f"tray-{name}.png")], check=True)

    def visible_windows(self):
        found = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(self.pid)], capture_output=True, text=True)
        return found.stdout.split()

    def has_name(self, name):
        return self.call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "NameHasOwner", GLib.Variant("(s)", (name,)))[0]

    def press_alt_f4_through_mutter(self):
        session = self.call(self.REMOTE_DESKTOP, "/org/gnome/Mutter/RemoteDesktop", self.REMOTE_DESKTOP, "CreateSession")[0]
        interface = f"{self.REMOTE_DESKTOP}.Session"
        self.call(self.REMOTE_DESKTOP, session, interface, "Start")
        try:
            for keysym, down in [(self.SHIFT, True), (self.SHIFT, False), (self.ALT, True), (self.F4, True), (self.F4, False), (self.ALT, False)]:
                self.call(self.REMOTE_DESKTOP, session, interface, "NotifyKeyboardKeysym", GLib.Variant("(ub)", (keysym, down)))
        finally:
            self.call(self.REMOTE_DESKTOP, session, interface, "Stop")

    def registered_items(self):
        items = self.call(self.WATCHER, "/StatusNotifierWatcher", self.PROPERTIES, "Get",
                          GLib.Variant("(ss)", (self.WATCHER, "RegisteredStatusNotifierItems")))[0]
        owner = self.call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "GetNameOwner",
                          GLib.Variant("(s)", (self.service,)))[0]
        return [item for item in items if self.service in item or item.startswith(f"{owner}/") or item == owner]

    def item_property(self, name):
        return self.call(self.service, self.ITEM_PATH, self.PROPERTIES, "Get", GLib.Variant("(ss)", (self.ITEM, name)))[0]

    def menu_entries(self):
        layout = self.call(self.service, self.MENU_PATH, self.MENU, "GetLayout", GLib.Variant("(iias)", (0, -1, [])))[1]
        return [(entry[0], entry[1].get("label", "")) for entry in layout[2] if entry[1].get("type") != "separator"]

    def menu_id(self, label):
        matches = [entry_id for entry_id, entry_label in self.menu_entries() if entry_label.startswith(label)]
        assert matches, f"the tray menu has no {label!r} entry: {self.menu_entries()}"
        return matches[0]

    def click(self, label):
        self.call(self.service, self.MENU_PATH, self.MENU, "Event",
                  GLib.Variant("(isvu)", (self.menu_id(label), "clicked", GLib.Variant("i", 0), 0)))

    def check_shown(self):
        self.wait(lambda: self.recorded("tray.shown") or self.recorded("tray.refused"), "the tray icon to register")
        assert self.recorded("tray.shown"), f"the desktop's StatusNotifierWatcher refused the icon: {self.events()}"
        self.wait(lambda: bool(self.registered_items()), f"{self.WATCHER} to list {self.service}")
        server = self.bus.call_sync("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "GetConnectionUnixProcessID",
                                    GLib.Variant("(s)", (self.WATCHER,)), None, Gio.DBusCallFlags.NONE, 5000, None).unpack()[0]
        host = Path(f"/proc/{server}/comm").read_text().strip()
        print(f"OK shown: the desktop's StatusNotifierWatcher ({host}, pid {server}) lists {self.registered_items()}", flush=True)
        print(f"OK icon: tooltip {self.item_property('ToolTip')[2]!r}, menu {[label for _, label in self.menu_entries()]}", flush=True)
        self.screenshot("shown")

    def check_close_hides(self):
        if self.on_x11:
            self.wait(lambda: bool(self.visible_windows()), "the app window")
            window = self.visible_windows()[0]
            subprocess.run(["xdotool", "windowactivate", "--sync", window, "key", "--clearmodifiers", "alt+F4"], check=True)
            self.wait(lambda: not self.visible_windows(), "Alt+F4 to hide the window")
            route = "Alt+F4 through the desktop's window manager"
        elif self.has_name(self.REMOTE_DESKTOP):
            self.wait(lambda: self.recorded("controller.mounted"), "the app window to load its page")
            self.press_alt_f4_through_mutter()
            self.wait(lambda: self.recorded("window.hidden"), "Alt+F4 to close the window (it hides only if the window had the keyboard focus)")
            route = "Alt+F4 through Mutter's RemoteDesktop keyboard"
        else:
            print(f"SKIP close: xdotool cannot press Alt+F4 on Wayland and the desktop has no {self.REMOTE_DESKTOP}, so the close-hides step did not run", flush=True)
            return
        time.sleep(1)
        assert self.running(), "closing the window ended the app"
        assert self.recorded("window.hidden"), f"the host did not record window.hidden: {self.events()}"
        print(f"OK close: {route} hid the window and the app kept running", flush=True)
        self.screenshot("hidden")

    def check_activate(self):
        self.call(self.service, self.ITEM_PATH, self.ITEM, "Activate", GLib.Variant("(ii)", (0, 0)))
        if not self.on_x11:
            print("OK activate: the icon answered its Activate call (on Wayland the window's return shows only in the screenshot)", flush=True)
            self.screenshot("activated")
            return
        self.wait(lambda: bool(self.visible_windows()), "Activate to show the window again")
        print("OK activate: the icon's Activate call showed the window again", flush=True)

    def check_route_item(self):
        self.click("Tray Help")
        self.wait(lambda: self.recorded("controller.mounted", "/help"), "the Tray Help item to open /help")
        print("OK route: the Tray Help menu item opened /help", flush=True)
        self.screenshot("help")

    def check_quit(self):
        self.click("Quit")
        self.wait(lambda: not self.running(), "the Quit item to end the app")
        print("OK quit: the Quit menu item ended the app", flush=True)

    def main(self):
        self.check_shown()
        self.check_close_hides()
        self.check_activate()
        self.check_route_item()
        self.check_quit()


if __name__ == "__main__":
    DesktopTrayProbe(int(sys.argv[1]), sys.argv[2], os.environ.get("TAC_LINUX_SCREENSHOTS", "")).main()
