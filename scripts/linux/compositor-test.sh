#!/usr/bin/env bash
set -euo pipefail

release_document_portal() {
  if mountpoint -q "$STATE/runtime/doc" 2>/dev/null; then
    fusermount3 -u "$STATE/runtime/doc" 2>/dev/null || fusermount -u "$STATE/runtime/doc" 2>/dev/null || true
  fi
}

remove_private_folders() {
  release_document_portal
  rm -rf "$STATE"
}

use_private_folders() {
  STATE=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-wayland.XXXXXX")
  export STATE
  trap remove_private_folders EXIT
  mkdir -m 700 "$STATE/runtime"
  export XDG_RUNTIME_DIR="$STATE/runtime"
  export XDG_DATA_HOME="$STATE/data"
  export XDG_CACHE_HOME="$STATE/cache"
  export XDG_STATE_HOME="$STATE/state"
  export XDG_CONFIG_HOME="$STATE/config"
}

build_tests() {
  echo '==> building the Linux host tests'
  cargo test --locked -p tachyon-core --lib --no-run
}

run_in_mutter() {
  echo '==> running the Linux host tests in a headless mutter Wayland session'
  export TAC_WAYLAND_DISPLAY_NAME=tachyon-wayland
  dbus-run-session -- bash -c '
    set -euo pipefail
    mutter --headless --wayland --no-x11 --wayland-display="$TAC_WAYLAND_DISPLAY_NAME" --virtual-monitor 1280x900 >"$STATE/mutter.log" 2>&1 &
    compositor=$!
    trap '\''kill "$compositor" 2>/dev/null || true; wait "$compositor" 2>/dev/null || true'\'' EXIT
    for _ in $(seq 60); do
      if [[ -S "$XDG_RUNTIME_DIR/$TAC_WAYLAND_DISPLAY_NAME" ]]; then break; fi
      kill -0 "$compositor" 2>/dev/null || { cat "$STATE/mutter.log" >&2; exit 1; }
      sleep 0.5
    done
    unset DISPLAY
    export WAYLAND_DISPLAY="$TAC_WAYLAND_DISPLAY_NAME" GDK_BACKEND=wayland
    export LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 GDK_DEBUG=no-portals
    export GIO_USE_VFS=local GIO_USE_VOLUME_MONITOR=unix
    mutter --version
    cargo test --locked -p tachyon-core --lib native::linux_ -- --ignored --test-threads=1 \
      --skip linux_browser_apis_reach_geoclue_and_the_notification_server --skip linux_package_tests
    cargo test --locked -p tachyon-core --lib the_linux_ -- --ignored --test-threads=1
  '
}

run_in_wslg() {
  local socket="${TAC_WSLG_WAYLAND_SOCKET:-/mnt/wslg/runtime-dir/wayland-0}"
  echo "==> running the Linux host tests in the WSLg Wayland session ($socket)"
  test -S "$socket" || { echo "WSLg has no Wayland socket at $socket" >&2; exit 1; }
  dbus-run-session -- env -u DISPLAY WAYLAND_DISPLAY="$socket" GDK_BACKEND=wayland \
    LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 GDK_DEBUG=no-portals bash -c '
    set -euo pipefail
    cargo test --locked -p tachyon-core --lib native::linux_ -- --ignored --test-threads=1 \
      --skip linux_browser_api_tests --skip linux_file_picker_tests --skip linux_package_tests \
      --skip linux_title_bar_tests
    cargo test --locked -p tachyon-core --lib the_linux_ -- --ignored --test-threads=1
  '
}

run_under_compositing_x11() {
  echo '==> running the title bar and window control tests under xfwm4 with its compositor on'
  GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 GDK_DEBUG=no-portals \
    dbus-run-session -- xvfb-run --auto-servernum --server-args='-screen 0 1280x1000x24 +extension Composite' bash -c '
      set -euo pipefail
      xfwm4 --compositor=on --sm-client-disable >"$STATE/xfwm4.log" 2>&1 &
      manager=$!
      trap '\''kill "$manager" 2>/dev/null || true; wait "$manager" 2>/dev/null || true'\'' EXIT
      for _ in $(seq 40); do
        if [[ "$(compositor_owner)" != 0 ]]; then break; fi
        sleep 0.25
      done
      test "$(compositor_owner)" != 0 || { echo "xfwm4 did not start compositing" >&2; cat "$STATE/xfwm4.log" >&2; exit 1; }
      echo "xfwm4 owns _NET_WM_CM_S0"
      cargo test --locked -p tachyon-core --lib native::linux_title_bar_tests -- --ignored --test-threads=1
      cargo test --locked -p tachyon-core --lib native::linux_window_control_tests -- --ignored --test-threads=1
    '
}

use_the_live_desktop() {
  local exports
  exports=$(node "$(dirname "${BASH_SOURCE[0]}")/linux-display-session.mjs" desktop-env)
  eval "$exports"
}

run_in_desktop() {
  echo "==> running the Linux host tests in the live $XDG_SESSION_TYPE desktop session (${XDG_CURRENT_DESKTOP:-unnamed desktop} on ${WAYLAND_DISPLAY:-$DISPLAY}, desktop services: ${TAC_DESKTOP_SERVICES:-none})"
  local skipped=(--skip linux_browser_apis_reach_geoclue_and_the_notification_server --skip linux_package_tests)
  DESKTOP_GAPS=
  if [[ $XDG_SESSION_TYPE == x11 ]]; then
    echo "_NET_WM_CM_S0 owner (0 when the desktop does not composite): $(compositor_owner)"
  elif [[ " $TAC_DESKTOP_SERVICES " != *' remote-desktop '* ]]; then
    echo '==> this Wayland desktop has no Mutter RemoteDesktop, so the browser API, file dialog and title bar tests, which press keys and the pointer through it, do not run'
    skipped+=(--skip linux_browser_api_tests --skip linux_file_picker_tests --skip linux_title_bar_tests)
    DESKTOP_GAPS=' (not run: the browser API, file dialog and title bar tests, because the desktop has no Mutter RemoteDesktop)'
  fi
  export LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 GDK_DEBUG=no-portals
  export GIO_USE_VFS=local GIO_USE_VOLUME_MONITOR=unix
  cargo test --locked -p tachyon-core --lib native::linux_ -- --ignored --test-threads=1 --nocapture "${skipped[@]}"
  cargo test --locked -p tachyon-core --lib the_linux_ -- --ignored --test-threads=1
}

compositor_owner() {
  python3 - <<'PY'
import ctypes
x11 = ctypes.CDLL("libX11.so.6")
x11.XOpenDisplay.restype = ctypes.c_void_p
x11.XInternAtom.restype = ctypes.c_ulong
x11.XGetSelectionOwner.restype = ctypes.c_ulong
display = ctypes.c_void_p(x11.XOpenDisplay(None))
atom = x11.XInternAtom(display, b"_NET_WM_CM_S0", 0)
print(x11.XGetSelectionOwner(display, ctypes.c_ulong(atom)))
PY
}

main() {
  use_private_folders
  build_tests
  if [[ "${1:-}" == --wslg ]]; then
    run_in_wslg
    echo 'PASS: the Linux host, window controls, appearance, storage, page calls and links under the WSLg Wayland compositor (the title bar tests press the pointer through Mutter RemoteDesktop, which WSLg does not have; they run under mutter and xfwm4)'
    return
  fi
  if [[ "${1:-}" == --desktop ]]; then
    use_the_live_desktop
    run_in_desktop
    echo "PASS: the Linux host, window controls, appearance, storage, page calls, links, the integrated title bar, the browser APIs and the file dialog in the live $XDG_SESSION_TYPE desktop session (${XDG_CURRENT_DESKTOP:-unnamed desktop}), under its own window manager and session bus$DESKTOP_GAPS"
    return
  fi
  export -f compositor_owner
  run_under_compositing_x11
  run_in_mutter
  echo 'PASS: the integrated title bar and window controls under a compositing X11 window manager, and the Linux host, window controls, appearance (size, theme_color, background_color), storage, page calls, links, the integrated title bar (insets, drag regions, double click), the browser APIs (pointer clicks) and the file dialog (key presses) under Wayland, with input through Mutter RemoteDesktop'
}

main "$@"
