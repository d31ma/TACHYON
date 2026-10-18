#!/usr/bin/env bash
set -euo pipefail
here="$(dirname "${BASH_SOURCE[0]}")"
node "$here/native-test.mjs" "$@"
node "$here/window-controls-test.mjs"
