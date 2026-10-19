#!/usr/bin/env bash
set -euo pipefail

use_private_folders() {
  STATE=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-wayland.XXXXXX")
  export STATE
  trap 'rm -rf "$STATE"' EXIT
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
      --skip linux_browser_api_tests --skip linux_package_tests
    cargo test --locked -p tachyon-core --lib the_linux_ -- --ignored --test-threads=1
  '
}

main() {
  use_private_folders
  build_tests
  run_in_mutter
  echo 'PASS: the Linux host, window controls, appearance, storage, page calls, links and the integrated title bar (insets, drag regions, double click) under Wayland'
}

main "$@"
