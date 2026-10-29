#!/usr/bin/env bash
set -euo pipefail

release_document_portal() {
  for _ in $(seq 40); do
    mountpoint -q "$STATE/runtime/doc" 2>/dev/null || return 0
    fusermount3 -uz "$STATE/runtime/doc" 2>/dev/null || fusermount -uz "$STATE/runtime/doc" 2>/dev/null || true
    sleep 0.25
  done
}

remove_private_folders() {
  release_document_portal
  for _ in $(seq 40); do
    rm -rf "$STATE" 2>/dev/null && return 0
    sleep 0.25
  done
  rm -rf "$STATE"
}

use_private_folders() {
  STATE=$(mktemp -d "${TMPDIR:-/tmp}/ty-gnome-shell.XXXXXX")
  export STATE
  trap remove_private_folders EXIT
  mkdir -m 700 "$STATE/runtime"
  export XDG_RUNTIME_DIR="$STATE/runtime"
  export XDG_CONFIG_HOME="$STATE/config"
}

stop_the_shell() {
  kill "$SHELL_PID" 2>/dev/null || true
  wait "$SHELL_PID" 2>/dev/null || true
}

shell_mode() {
  if [[ -f /usr/share/gnome-shell/modes/ubuntu.json ]]; then echo ubuntu; else echo user; fi
}

shell_is_on_the_bus() {
  gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus -m org.freedesktop.DBus.NameHasOwner org.gnome.Shell 2>/dev/null | grep -q true
}

overview_is_open() {
  gdbus call --session -d org.gnome.Shell -o /org/gnome/Shell -m org.freedesktop.DBus.Properties.Get org.gnome.Shell OverviewActive 2>/dev/null | grep -q true
}

leave_the_overview() {
  local closed=0
  for _ in $(seq 40); do
    gdbus call --session -d org.gnome.Shell -o /org/gnome/Shell -m org.freedesktop.DBus.Properties.Set org.gnome.Shell OverviewActive '<false>' >/dev/null 2>&1 || true
    sleep 0.25
    if overview_is_open; then closed=0; else closed=$((closed + 1)); fi
    if [[ $closed == 4 ]]; then
      echo '==> GNOME Shell left the overview it opens at startup, so new windows take the focus'
      return
    fi
  done
  echo '==> GNOME Shell stayed in its overview; new windows may not take the focus' >&2
}

start_the_shell() {
  local mode
  mode=$(shell_mode)
  unset DISPLAY
  export WAYLAND_DISPLAY=tachyon-gnome-shell XDG_SESSION_TYPE=wayland GDK_BACKEND=wayland
  export XDG_CURRENT_DESKTOP="${TAC_GNOME_SHELL_DESKTOP:-ubuntu:GNOME}" XDG_SESSION_DESKTOP="$mode" DESKTOP_SESSION="$mode"
  dbus-update-activation-environment WAYLAND_DISPLAY XDG_SESSION_TYPE GDK_BACKEND XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP DESKTOP_SESSION XDG_RUNTIME_DIR XDG_CONFIG_HOME
  gsettings set org.gnome.desktop.interface enable-hot-corners false
  gnome-shell --headless --wayland --no-x11 --wayland-display="$WAYLAND_DISPLAY" --virtual-monitor "${TAC_GNOME_SHELL_SIZE:-1280x900}" --mode="$mode" >"$STATE/gnome-shell.log" 2>&1 &
  SHELL_PID=$!
  trap stop_the_shell EXIT
  for _ in $(seq 120); do
    if [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && shell_is_on_the_bus; then break; fi
    kill -0 "$SHELL_PID" 2>/dev/null || { cat "$STATE/gnome-shell.log" >&2; exit 1; }
    sleep 0.5
  done
  shell_is_on_the_bus || { echo 'GNOME Shell did not take org.gnome.Shell on its session bus within 60 s' >&2; cat "$STATE/gnome-shell.log" >&2; exit 1; }
  leave_the_overview
  echo "==> $(gnome-shell --version) runs headless on the Wayland display $WAYLAND_DISPLAY (mode $mode, process $SHELL_PID)"
  echo "enabled GNOME Shell extensions: $(gnome-extensions list --enabled 2>/dev/null | tr '\n' ' ' || true)"
}

run_in_the_shell() {
  local status=0
  TAC_LINUX_DISPLAY=desktop TAC_LINUX_DESKTOP_PID="$SHELL_PID" "$@" || status=$?
  if [[ $status != 0 ]]; then
    echo "==> the command failed with status $status; the end of the GNOME Shell log:" >&2
    tail -n 40 "$STATE/gnome-shell.log" >&2
  fi
  return "$status"
}

main() {
  if [[ $# == 0 ]]; then
    echo 'usage: gnome-shell-session.sh <command> [arguments], for example scripts/linux/compositor-test.sh --desktop' >&2
    exit 2
  fi
  if [[ $1 != --inside-the-bus ]]; then
    use_private_folders
    dbus-run-session -- "$0" --inside-the-bus "$@"
    return
  fi
  shift
  start_the_shell
  run_in_the_shell "$@"
}

main "$@"
