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
    mutter --version
    cargo test --locked -p tachyon-core --lib native::linux_ -- --ignored --test-threads=1 \
      --skip linux_browser_api_tests --skip linux_file_picker_tests --skip linux_package_tests
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
  export -f compositor_owner
  run_under_compositing_x11
  run_in_mutter
  echo 'PASS: the integrated title bar and window controls under a compositing X11 window manager, and the Linux host, window controls, appearance, storage, page calls, links and the integrated title bar (insets, drag regions, double click) under Wayland'
}

main "$@"
