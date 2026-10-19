#!/usr/bin/env bash
set -euo pipefail
here="$(dirname "${BASH_SOURCE[0]}")"
node "$here/native-test.mjs" "$@"
node "$here/window-controls-test.mjs"
node "$here/window-settings-test.mjs"
node "$here/browser-apis-test.mjs"
node "$here/yon-origin-test.mjs"
node "$here/rust-services-test.mjs"
