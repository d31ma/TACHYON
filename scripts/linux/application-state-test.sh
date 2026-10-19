#!/usr/bin/env bash
set -euo pipefail

write_fixture() {
  local root=$1
  mkdir -p "$root/client/pages" "$root/client/components/state/panel" "$root/client/services"
  cat > "$root/client/manifest.json" <<'JSON'
{
  "name": "State Gate",
  "start_url": "/",
  "tac": { "application": { "id": "dev.tachyon.state-gate", "version": "0.0.1" } }
}
JSON
  echo '<state-panel />' > "$root/client/pages/tac.html"
  echo '<main aria-label="State"><h1>State {stateWatch.state}</h1></main>' > "$root/client/components/state/panel/tac.html"
  cat > "$root/client/services/StateWatchService.js" <<'JS'
@Service
export default class StateWatchService {
  state = 'unknown'

  /** @param {ApplicationStateChanged} changed */
  @OnEvent({"replay":true})
  heard(changed) {
    this.state = changed.state
    void this.pageHeard(changed.state)
  }
}
JS
  cat > "$root/client/services/StateWatchService.rs" <<'RS'
#[Service]
#[derive(Default)]
struct StateWatchService {
    count: i64,
}

impl StateWatchService {
    pub fn page_heard(&self, state: String) -> i64 {
        Self::write(format!("page:{state}"));
        self.count
    }

    #[OnEvent({"replay":true})]
    fn native_heard(&mut self, changed: ApplicationStateChanged) {
        self.count += 1;
        Self::write(format!("native:{}", changed.state));
    }

    fn write(line: String) {
        use std::io::Write;
        let path = std::env::var("STATE_GATE_LOG").unwrap_or_default();
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}
RS
}

build_native_app() {
  local fixture=$1
  if [[ -z "${TAC_BIN:-}" ]]; then
    cargo build --locked --bin ty
    TAC_BIN="${CARGO_TARGET_DIR:-target}/debug/ty"
  fi
  echo '==> building the Linux host with a page service and a linked Rust service'
  timeout 300s "$TAC_BIN" build "$fixture" --target linux
}

drive_application_state() {
  local app=$1
  echo '==> sending the app to the background and back under Xvfb and xfwm4'
  export GDK_BACKEND=x11
  export LIBGL_ALWAYS_SOFTWARE=1
  export WEBKIT_DMABUF_RENDERER_FORCE_SHM=1
  export APP="$app"
  timeout 150s xvfb-run --auto-servernum --server-args='-screen 0 1280x1024x24' \
    dbus-run-session -- bash -c '
      set -euo pipefail
      xfwm4 --compositor=off >/dev/null 2>&1 &
      sleep 2
      "$APP/bin/StateGate" >/dev/null 2>&1 &
      host=$!
      trap '\''kill $(jobs -p) 2>/dev/null || true; wait "$host" 2>/dev/null || true'\'' EXIT
      wait_for() { for _ in $(seq 1 40); do grep -qx "$1" "$STATE_GATE_LOG" 2>/dev/null && return 0; sleep 0.5; done; echo "missing $1"; cat "$STATE_GATE_LOG" 2>/dev/null; return 1; }
      wait_for native:foreground
      xmessage -geometry 300x100+600+500 "Other app" >/dev/null 2>&1 &
      sleep 2
      xdotool search --sync --class xmessage windowactivate
      wait_for native:background
      wait_for page:background
      : > "$STATE_GATE_LOG"
      xdotool search --name "State Gate" windowactivate
      wait_for native:foreground
      wait_for page:foreground
      : > "$STATE_GATE_LOG"
      xdotool search --name "State Gate" windowminimize
      wait_for native:background
    '
}

check_lifecycle_log() {
  local log="$XDG_STATE_HOME/tachyon/dev.tachyon.state-gate.jsonl"
  local event
  for event in application.background application.foreground; do
    grep -q "\"event\":\"$event\"" "$log"
  done
}

main() {
  local tool
  for tool in xvfb-run dbus-run-session xfwm4 xdotool xmessage; do
    command -v "$tool" >/dev/null || { echo "FAIL: $tool is required"; exit 1; }
  done
  FIXTURE=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-state.XXXXXX")
  trap 'rm -rf "$FIXTURE"' EXIT
  write_fixture "$FIXTURE"
  build_native_app "$FIXTURE"
  export STATE_GATE_LOG="$FIXTURE/state.log"
  export XDG_STATE_HOME="$FIXTURE/state"
  export XDG_DATA_HOME="$FIXTURE/data"
  drive_application_state "$FIXTURE/dist/linux/StateGate"
  check_lifecycle_log
  echo 'PASS: the Linux host sends background and foreground to the page and to a linked Rust service'
}

main "$@"
