#!/usr/bin/env bash
set -euo pipefail

PACKAGE=nativegate
DESKTOP_FILE=/usr/share/applications/dev.tachyon.desktop_gate.desktop

need_root() {
  if [[ "$(id -u)" != 0 ]]; then
    echo 'This test installs the .deb with apt, so run it as root (for example in a container or VM).' >&2
    exit 1
  fi
  if dpkg -s "$PACKAGE" >/dev/null 2>&1; then
    echo "$PACKAGE is already installed here; remove it before this test." >&2
    exit 1
  fi
}

in_the_live_desktop() {
  [[ ${TAC_LINUX_DISPLAY:-x11} == desktop ]]
}

use_the_live_desktop() {
  if [[ -z "${TAC_LINUX_DESKTOP_USER:-}" ]]; then
    echo 'TAC_LINUX_DISPLAY=desktop needs TAC_LINUX_DESKTOP_USER, the user signed in to the desktop, because apt runs as root and the app runs as that user' >&2
    exit 1
  fi
  local exports
  exports=$(node "$(dirname "$0")/linux-display-session.mjs" desktop-env)
  eval "$exports"
  local folder
  for folder in "$XDG_STATE_HOME" "$XDG_DATA_HOME" "$XDG_CACHE_HOME" "$XDG_CONFIG_HOME"; do
    mkdir -p "$folder"
    chown "$TAC_LINUX_DESKTOP_USER" "$folder"
  done
  chmod 711 "$WORK"
  echo "==> starting the installed app as $TAC_LINUX_DESKTOP_USER in the live $XDG_SESSION_TYPE desktop session (${XDG_CURRENT_DESKTOP:-unnamed desktop} on ${WAYLAND_DISPLAY:-$DISPLAY})"
}

beside_display() {
  if in_the_live_desktop; then
    LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 timeout 150s runuser -u "$TAC_LINUX_DESKTOP_USER" -- "$@"
    return
  fi
  GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 \
    timeout 150s xvfb-run --auto-servernum --server-args='-screen 0 1024x1400x24' dbus-run-session -- "$@"
}

remove_folders() {
  if dpkg -s "$PACKAGE" >/dev/null 2>&1; then apt-get purge -y "$PACKAGE" >/dev/null || true; fi
  rm -rf "$WORK"
}

use_private_folders() {
  WORK=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-deb.XXXXXX")
  trap remove_folders EXIT
  FIXTURE="$WORK/app"
  mkdir "$FIXTURE"
  mkdir -m 700 "$WORK/runtime"
  export XDG_RUNTIME_DIR="$WORK/runtime"
  export XDG_STATE_HOME="$WORK/state"
  export XDG_DATA_HOME="$WORK/data"
  export XDG_CACHE_HOME="$WORK/cache"
  export XDG_CONFIG_HOME="$WORK/config"
  unset WAYLAND_DISPLAY
}

build_release_deb() {
  python3 scripts/native/desktop-fixture.py "$FIXTURE" linux
  if [[ -z "${TAC_BIN:-}" ]]; then
    cargo build --locked --bin ty
    TAC_BIN="${CARGO_TARGET_DIR:-target}/debug/ty"
  fi
  echo '==> building the Linux release packages'
  timeout 300s "$TAC_BIN" bundle "$FIXTURE" --target linux --release --out-dir release
  DEB=$(find "$FIXTURE/release/linux" -maxdepth 1 -name "${PACKAGE}_0.0.1-7_*.deb" -print -quit)
  test -f "$DEB"
  dpkg-deb --field "$DEB" Package Version Architecture Depends
}

install_deb() {
  echo "==> installing $(basename "$DEB") with apt as root"
  apt-get install -y "$DEB"
  dpkg -s "$PACKAGE" | grep -qx 'Status: install ok installed'
  test "$(command -v "$PACKAGE")" = "/usr/bin/$PACKAGE"
  test -f "/usr/lib/$PACKAGE/bin/libtachyoncompanion.so"
  test -f "$DESKTOP_FILE"
  desktop-file-validate "$DESKTOP_FILE"
  grep -qx "Exec=$PACKAGE" "$DESKTOP_FILE"
  echo "installed: $(dpkg-query -W -f='${Package} ${Version} ${Architecture}' "$PACKAGE"), $(dpkg -L "$PACKAGE" | wc -l) paths"
}

launch_until_mounted() {
  local label=$1
  shift
  echo "==> starting the installed app through $label and waiting for its first page and Rust library"
  local log="$XDG_STATE_HOME/tachyon/dev.tachyon.desktop-gate.jsonl"
  rm -f "$log"
  export LOG="$log"
  beside_display bash -c '
      set -euo pipefail
      "$@" &
      launcher=$!
      trap '\''pkill -x NativeGate 2>/dev/null || true; kill "$launcher" 2>/dev/null || true; wait 2>/dev/null || true'\'' EXIT
      for _ in $(seq 120); do
        if grep -q "\"event\":\"controller.mounted\"" "$LOG" 2>/dev/null && grep -q "\"event\":\"companion.loaded\"" "$LOG"; then exit 0; fi
        sleep 1
      done
      echo "the installed app showed no page in 120 seconds" >&2
      exit 1
    ' launch "$@"
}

launch_from_the_desktop_menu() {
  local launcher
  launcher=$(command -v gtk4-launch || command -v gtk-launch || true)
  if [[ -z "$launcher" ]]; then
    echo 'SKIP: neither gtk4-launch nor gtk-launch is installed, so the app was not started by its desktop file id'
    return
  fi
  echo "the desktop's app menu entry: $(grep -E '^(Name|Categories)=' "$DESKTOP_FILE" | tr '\n' ' ')"
  launch_until_mounted "its desktop file id, as the desktop's app menu finds it ($(basename "$launcher"))" "$launcher" "$(basename "$DESKTOP_FILE" .desktop)"
}

purge_deb() {
  echo "==> removing $PACKAGE with apt purge"
  apt-get purge -y "$PACKAGE"
  if dpkg -s "$PACKAGE" >/dev/null 2>&1; then echo "$PACKAGE is still installed" >&2; exit 1; fi
  test ! -e "/usr/bin/$PACKAGE"
  test ! -e "/usr/lib/$PACKAGE"
  test ! -e "$DESKTOP_FILE"
}

main() {
  need_root
  use_private_folders
  if in_the_live_desktop; then use_the_live_desktop; fi
  build_release_deb
  install_deb
  launch_until_mounted "the command on PATH" "$PACKAGE"
  launch_until_mounted "its installed .desktop file" gio launch "$DESKTOP_FILE"
  if in_the_live_desktop; then launch_from_the_desktop_menu; fi
  purge_deb
  echo "PASS: apt installed the release .deb as root, the app started from PATH and from its installed .desktop file (display: ${TAC_LINUX_DISPLAY:-x11}), and apt purge removed it"
}

main "$@"
