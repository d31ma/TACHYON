#!/usr/bin/env python3

import os
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402


class ProbeLimits:
    MAX_NODES = 4096
    MAX_DEPTH = 24
    MAX_CHILDREN = 128
    MAX_TEXT_LENGTH = 512
    WAIT_SECONDS = 20
    APPLICATION = os.environ.get("TAC_PROBE_APPLICATION", "NativeGate")
    POLL_SECONDS = 0.2


class KeySymbols:
    F10 = 0xFFC7
    ESCAPE = 0xFF1B


class ExpectedRoles:
    WINDOW = [(Atspi.Role.FRAME, "Native Gate"),
              (Atspi.Role.MENU_ITEM, "File"),
              (Atspi.Role.MENU_ITEM, "Edit"),
              (Atspi.Role.MENU_ITEM, "Window")]
    ROOT_PAGE = [(Atspi.Role.LANDMARK, "Root route"),
                 (Atspi.Role.HEADING, "Root route"),
                 (Atspi.Role.PUSH_BUTTON, "Verify native Root"),
                 (Atspi.Role.LINK, "Open second"),
                 (Atspi.Role.LINK, "Leave app")]
    FILE_MENU = [(Atspi.Role.MENU_ITEM, "Close Window"),
                 (Atspi.Role.MENU_ITEM, "Quit")]


def walk_tree(root):
    pending = [(root, 0)]
    for _ in range(ProbeLimits.MAX_NODES):
        if not pending:
            return
        node, depth = pending.pop()
        if node is None:
            continue
        yield node
        if depth >= ProbeLimits.MAX_DEPTH:
            continue
        for index in range(min(node.get_child_count(), ProbeLimits.MAX_CHILDREN)):
            pending.append((node.get_child_at_index(index), depth + 1))


def find_application():
    desktop = Atspi.get_desktop(0)
    for index in range(min(desktop.get_child_count(), ProbeLimits.MAX_CHILDREN)):
        candidate = desktop.get_child_at_index(index)
        if candidate is not None and candidate.get_name() == ProbeLimits.APPLICATION:
            return candidate
    return None


def exposed_texts(root):
    result = set()
    for node in walk_tree(root):
        result.add(node.get_name())
        text = node.get_text_iface()
        if text is None:
            continue
        end = min(Atspi.Text.get_character_count(text), ProbeLimits.MAX_TEXT_LENGTH)
        result.add(Atspi.Text.get_text(text, 0, end))
    return result


def wait_for_controls(required, seconds=ProbeLimits.WAIT_SECONDS):
    deadline = time.monotonic() + seconds
    exposed = set()
    while time.monotonic() < deadline:
        root = find_application()
        if root is not None:
            exposed = exposed_texts(root)
            if set(required) <= exposed:
                print("OK:", ", ".join(required), flush=True)
                return root
        time.sleep(ProbeLimits.POLL_SECONDS)
    raise AssertionError(f"Missing {set(required) - exposed}; exposed: {sorted(exposed)}")


def exposed_roles(root):
    return {(node.get_role(), node.get_name()) for node in walk_tree(root)}


def wait_for_roles(required, seconds=ProbeLimits.WAIT_SECONDS):
    deadline = time.monotonic() + seconds
    exposed = set()
    while time.monotonic() < deadline:
        root = find_application()
        if root is not None:
            exposed = exposed_roles(root)
            if set(required) <= exposed:
                names = [f"{role.value_nick} {name!r}" for role, name in required]
                print("OK roles:", ", ".join(names), flush=True)
                return root
        time.sleep(ProbeLimits.POLL_SECONDS)
    raise AssertionError(f"Missing roles {set(required) - exposed}; exposed: {sorted(exposed)}")


def open_file_menu():
    wait_for_roles([(Atspi.Role.MENU_ITEM, "File")])
    Atspi.generate_keyboard_event(KeySymbols.F10, None, Atspi.KeySynthType.SYM)
    wait_for_roles(ExpectedRoles.FILE_MENU)
    Atspi.generate_keyboard_event(KeySymbols.ESCAPE, None, Atspi.KeySynthType.SYM)


def run_first_action(node):
    action = node.get_action_iface()
    if action is None:
        return False
    if not action.get_n_actions():
        return False
    return bool(action.do_action(0))


def activate_control(name):
    root = wait_for_controls([name])
    for node in walk_tree(root):
        if node.get_name() != name:
            continue
        if run_first_action(node):
            return
    raise AssertionError(f"Rendered control {name!r} has no accessible activation action")


def main():
    Atspi.init()
    wait_for_controls(["Root route", "Root count 0", "Shared module ready", "Verify native Root"])
    wait_for_roles(ExpectedRoles.WINDOW + ExpectedRoles.ROOT_PAGE)
    open_file_menu()
    activate_control("Verify native Root")
    wait_for_controls(["Root count 7", "Native Rust 14", "OS ready", "Styles ready",
                       "Route boundary ready", "Publish received"])
    activate_control("Open second")
    wait_for_controls(["Second route", "Second count 0", "Shared module ready"])
    activate_control("Verify native Second")
    wait_for_controls(["Second count 9", "Native Rust 18", "Styles ready",
                       "Route boundary ready", "Publish received"])
    activate_control("Return root")
    wait_for_controls(["Root route", "Root count 0"])
    activate_control("Leave app")
    time.sleep(0.5)
    wait_for_controls(["Root route", "Root count 0", "Verify native Root"])
    print("PASS: rendered WebView/native interaction, roles, menus and navigation boundary", flush=True)


if __name__ == "__main__":
    main()
