#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RELEASES_URL="https://github.com/d31ma/TACHYON/releases"

usage() {
  cat <<EOF
Usage: scripts/release/lifecycle-drill.sh [--from VERSION] [--candidate ARCHIVE] [--log-dir DIR]

Installs the published release VERSION from its release archive, runs a sample
app, upgrades to the candidate build and migrates the app, checks the app's data
and config survive, rolls back to VERSION from its archive with the source
reverted, and uninstalls.

  --from VERSION       Published release to start from and roll back to.
                       Defaults to TAC_DRILL_FROM, else the latest published release.
  --candidate ARCHIVE  Candidate release archive (tachyon-<version>-<arch>-<os>.tar.gz).
                       Without it, scripts/release/build-artifact.sh builds the
                       candidate twice from this checkout and checks it is reproducible.
  --log-dir DIR        Where the drill log and server logs go.
                       Defaults to \$LOG_ROOT/TACHYON/lifecycle-drill-<UTC time>.

TAC_DRILL_RELEASE_URL overrides the download folder of the --from release.
EOF
}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

stamp() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

run_step() {
  local name=$1
  shift
  echo "=== ${name} $(stamp)"
  local rc=0
  set +e
  (
    set -euo pipefail
    "$@"
  )
  rc=$?
  set -e
  echo "=== ${name} rc=${rc} $(stamp)"
  [ "${rc}" -eq 0 ] || exit "${rc}"
}

checksum() {
  if command -v sha256sum > /dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

verify_checksums() {
  if command -v sha256sum > /dev/null 2>&1; then
    (cd "$1" && sha256sum --quiet --check SHA256SUMS)
  else
    (cd "$1" && shasum -a 256 --quiet --check SHA256SUMS)
  fi
}

platform_os() {
  case "$(uname -s)" in
    Darwin) echo macos ;;
    Linux) echo linux ;;
    *) fail "the drill runs on macOS or Linux; Windows rollback is covered by install.ps1" ;;
  esac
}

platform_arch() {
  case "$(uname -m)" in
    arm64 | aarch64) echo aarch64 ;;
    *) echo x86_64 ;;
  esac
}

latest_release() {
  local location
  location="$(curl -fsSI "${RELEASES_URL}/latest" | tr -d '\r' | awk 'tolower($1) == "location:" { print $2 }' | tail -1)"
  [ -n "${location}" ] || fail "could not find the latest published release"
  local version
  version="$(basename "${location}" | sed 's/^v//')"
  echo "${version}" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || fail "the latest published release is '${version}', not a version"
  echo "${version}"
}

free_port() {
  node -e 'const server = require("node:net").createServer(); server.listen(0, "127.0.0.1", () => { console.log(server.address().port); server.close() })'
}

fingerprint() {
  local project=$1
  (cd "${project}" && find . \( -path ./dist -o -path ./.git -o -path ./node_modules \) -prune -o -type f -print | LC_ALL=C sort | while IFS= read -r file; do
    printf '%s  %s\n' "$(checksum "${file}")" "${file}"
  done)
}

kept_fingerprint() {
  local project=$1
  (cd "${project}" && for file in server/data/notes.json .env package.json; do
    printf '%s  %s\n' "$(checksum "${file}")" "${file}"
  done)
}

fetch_previous() {
  local version=$1
  local work=$2
  local base="${TAC_DRILL_RELEASE_URL:-${RELEASES_URL}/download/v${version}}"
  local name
  name="tachyon-${version}-$(platform_arch)-$(platform_os)"
  echo "    fetching ${base}/${name}.tar.gz"
  mkdir -p "${work}/download" "${work}/previous"
  curl -fsSL "${base}/${name}.tar.gz" -o "${work}/download/${name}.tar.gz"
  curl -fsSL "${base}/SHA256SUMS" -o "${work}/download/SHA256SUMS"
  local expected
  expected="$(awk -v asset="${name}.tar.gz" '$2 == asset { print $1; exit }' "${work}/download/SHA256SUMS")"
  [ -n "${expected}" ] || fail "release ${version} publishes no checksum for ${name}.tar.gz"
  [ "$(checksum "${work}/download/${name}.tar.gz")" = "${expected}" ] \
    || fail "the ${version} archive does not match the release SHA256SUMS"
  echo "    ok   the ${version} archive matches the release SHA256SUMS"
  tar -xzf "${work}/download/${name}.tar.gz" -C "${work}/previous"
  verify_checksums "${work}/previous/${name}" || fail "the ${version} archive contents do not match its SHA256SUMS"
  echo "    ok   every file in the ${version} archive matches its SHA256SUMS"
  ln -s "${name}" "${work}/previous/artifact"
}

build_candidate() {
  local work=$1
  echo "    building the candidate twice from ${REPO_ROOT}"
  "${REPO_ROOT}/scripts/release/build-artifact.sh" "${work}/build-a"
  "${REPO_ROOT}/scripts/release/build-artifact.sh" "${work}/build-b"
  local first
  first="$(find "${work}/build-a" -maxdepth 1 -type f -name 'tachyon-*.tar.gz' | head -1)"
  local second
  second="$(find "${work}/build-b" -maxdepth 1 -type f -name 'tachyon-*.tar.gz' | head -1)"
  [ -n "${first}" ] && [ -n "${second}" ] || fail "the candidate archive was not produced"
  [ "$(checksum "${first}")" = "$(checksum "${second}")" ] \
    || fail "two builds of one commit produced different release archives"
  echo "    ok   the candidate archive is bit-identical across two packagings"
  cp "${first}" "${work}/candidate.tar.gz"
}

unpack_candidate() {
  local archive=$1
  local work=$2
  [ -f "${archive}" ] || fail "no candidate archive at ${archive}"
  mkdir -p "${work}/candidate"
  tar -xzf "${archive}" -C "${work}/candidate"
  local unpacked
  unpacked="$(find "${work}/candidate" -mindepth 1 -maxdepth 1 -type d -name 'tachyon-*' | head -1)"
  [ -n "${unpacked}" ] || fail "the candidate archive holds no tachyon-* folder"
  ln -s "$(basename "${unpacked}")" "${work}/candidate/artifact"
  verify_checksums "${work}/candidate/artifact" || fail "the candidate archive contents do not match its SHA256SUMS"
  echo "    ok   every file in the candidate archive matches its SHA256SUMS"
  cp -R "${unpacked}" "${work}/tampered"
  printf '\0' >> "${work}/tampered/bin/ty"
  if verify_checksums "${work}/tampered" > /dev/null 2>&1; then
    fail "a modified candidate binary passed checksum verification"
  fi
  echo "    ok   a modified candidate fails verification"
  local previous_sum
  previous_sum="$(checksum "${work}/previous/artifact/bin/ty")"
  [ "$(checksum "${work}/candidate/artifact/bin/ty")" != "${previous_sum}" ] \
    || fail "the candidate binary is the previous release's binary"
  echo "    ok   the candidate binary differs from the previous release"
  local previous_version
  previous_version="$(node -p 'require(process.argv[1]).release_version' "${work}/previous/artifact/manifest.json")"
  local candidate_version
  candidate_version="$(node -p 'require(process.argv[1]).release_version' "${work}/candidate/artifact/manifest.json")"
  if [ -e "${work}/from-latest" ] && [ "${candidate_version}" = "${previous_version}" ]; then
    fail "the latest published release is already ${candidate_version}; pass --from with the release before it"
  fi
  echo "    candidate manifest:"
  sed 's/^/      /' "${work}/candidate/artifact/manifest.json"
}

install_binary() {
  local source=$1
  local prefix=$2
  mkdir -p "${prefix}/bin"
  local staged
  staged="$(mktemp "${prefix}/bin/.ty.download.XXXXXX")"
  cp "${source}" "${staged}"
  chmod 0755 "${staged}"
  mv "${staged}" "${prefix}/bin/ty"
}

check_installed() {
  local artifact=$1
  local prefix=$2
  local expected_version
  expected_version="$(node -p 'require(process.argv[1]).release_version' "${artifact}/manifest.json")"
  local installed
  installed="$("${prefix}/bin/ty" --version)" || fail "the installed binary does not run"
  [ "${installed}" = "${expected_version}" ] \
    || fail "installed version '${installed}' does not match the archive manifest '${expected_version}'"
  [ "$(checksum "${prefix}/bin/ty")" = "$(checksum "${artifact}/bin/ty")" ] \
    || fail "the installed binary is not the archive's binary"
  echo "    ok   installed ${installed} ($(checksum "${prefix}/bin/ty" | cut -c1-12))"
}

write_sample_app() {
  local prefix=$1
  local project=$2
  "${prefix}/bin/ty" init "${project}" --name "Lifecycle Drill"
  printf '{"notes":["written before the upgrade"],"schema":1}\n' > "${project}/server/data/notes.json"
  printf 'YON_DRILL_SECRET=kept\n' > "${project}/.env"
  cat > "${project}/server/routes/yon.js" <<'EOF'
@Controller
export class RootController {
  static async GET() {
    const { readFile } = await import('node:fs/promises')
    return JSON.parse(await readFile(new URL('../data/notes.json', import.meta.url), 'utf8'))
  }
}
EOF
}

commit_sample_app() {
  local project=$1
  git -C "${project}" init -q
  git -C "${project}" add -A
  git -C "${project}" -c user.name="Lifecycle Drill" -c user.email=drill@localhost commit -q -m "before the upgrade"
  echo "    committed $(git -C "${project}" ls-files | wc -l | tr -d ' ') tracked files as the pre-upgrade source"
}

migrate_sample_app() {
  local prefix=$1
  local project=$2
  "${prefix}/bin/ty" migrate fix "${project}" --write || fail "the candidate's ty migrate fix --write failed"
  if [ -f "${project}/server/routes/yon.js" ]; then
    mkdir -p "${project}/server/controllers"
    cat > "${project}/server/controllers/HomeController.js" <<'EOF'
@RequestRoute("/")
@Public
export class HomeController {
  @GetRoute
  async get() {
    const { readFile } = await import('node:fs/promises')
    return JSON.parse(await readFile(new URL('../data/notes.json', import.meta.url), 'utf8'))
  }
}
EOF
    rm -r "${project}/server/routes"
    echo "    manual step: moved server/routes/yon.js to server/controllers/HomeController.js"
  fi
  if [ -f "${project}/client/pages/tac.js" ] || [ -f "${project}/client/pages/tac.css" ]; then
    mkdir -p "${project}/client/components/shell" "${project}/client/shared/styles"
    if [ -f "${project}/client/pages/tac.html" ]; then
      mv "${project}/client/pages/tac.html" "${project}/client/components/shell/tac.html"
    fi
    printf '<shell />\n' > "${project}/client/pages/tac.html"
    if [ -f "${project}/client/pages/tac.css" ]; then
      cat "${project}/client/pages/tac.css" >> "${project}/client/shared/styles/app.css"
    fi
    rm -f "${project}/client/pages/tac.css" "${project}/client/pages/tac.js"
    echo "    manual step: moved the page markup into client/components/shell and its companions out of client/pages"
  fi
  git -C "${project}" status --short | sed 's/^/      /'
}

revert_sample_app() {
  local project=$1
  git -C "${project}" checkout -- .
  git -C "${project}" clean -fd -e dist
  echo "    reverted the migration's source changes with git checkout and git clean"
}

stop_server() {
  local server=$1
  kill -TERM "${server}" 2> /dev/null || return 0
  for _ in $(seq 1 30); do
    kill -0 "${server}" 2> /dev/null || return 0
    sleep 0.5
  done
  kill -KILL "${server}" 2> /dev/null || true
  return 1
}

show_server_log() {
  echo "    last lines of $1:" >&2
  tail -20 "$1" | sed 's/^/      /' >&2
}

serve_and_check() {
  local prefix=$1
  local project=$2
  local logs=$3
  local label=$4
  "${prefix}/bin/ty" bundle "${project}" || fail "${label} cannot bundle the sample app"
  local page
  page="$(find "${project}/dist" -name index.html -type f | head -1)"
  [ -n "${page}" ] || fail "${label} published no index.html"
  echo "    ok   ${label} bundled the sample app"
  local port
  local server
  local body
  local tries
  local attempt=1
  while :; do
    port="$(free_port)"
    "${prefix}/bin/ty" start "${project}" --port "${port}" > "${logs}/start-${label}.log" 2>&1 &
    server=$!
    trap 'stop_server '"${server}"' || true' EXIT
    body=""
    tries=0
    until body="$(curl -fsS "http://127.0.0.1:${port}/" 2> /dev/null)"; do
      if ! kill -0 "${server}" 2> /dev/null; then
        if [ "${attempt}" = 1 ] && [ -z "${YON_EGRESS_ENFORCEMENT:-}" ] && grep -q TY2113 "${logs}/start-${label}.log"; then
          show_server_log "${logs}/start-${label}.log"
          echo "    note ${label}: this host cannot confine handler egress (TY2113), so ty start runs again with YON_EGRESS_ENFORCEMENT=proxy"
          export YON_EGRESS_ENFORCEMENT=proxy
          attempt=2
          continue 2
        fi
        show_server_log "${logs}/start-${label}.log"
        fail "${label} ty start exited early"
      fi
      tries=$((tries + 1))
      if [ "${tries}" -ge 120 ]; then
        show_server_log "${logs}/start-${label}.log"
        fail "${label} ty start did not answer within 60 s"
      fi
      sleep 0.5
    done
    break
  done
  echo "    ok   ${label} answered with handler egress enforcement ${YON_EGRESS_ENFORCEMENT:-os}"
  node -e 'const served = JSON.parse(process.argv[1]); const stored = JSON.parse(require("node:fs").readFileSync(process.argv[2], "utf8")); process.exit(JSON.stringify(served) === JSON.stringify(stored) ? 0 : 1)' \
    "${body}" "${project}/server/data/notes.json" || fail "${label} served '${body}', not the stored data"
  echo "    ok   ${label} served the stored data: ${body}"
  trap - EXIT
  stop_server "${server}" || fail "${label} ty start did not stop within 15 s of SIGTERM, so it was killed"
  wait "${server}" 2> /dev/null || true
  echo "    ok   ${label} stopped on SIGTERM"
}

check_project_unchanged() {
  local project=$1
  local expected=$2
  local label=$3
  local actual
  actual="$(fingerprint "${project}")"
  if [ "${actual}" != "$(cat "${expected}")" ]; then
    diff <(cat "${expected}") <(printf '%s\n' "${actual}") >&2 || true
    fail "${label} changed the project's data or config"
  fi
  echo "    ok   the project's data and config are byte-identical after ${label}"
}

step_install_previous() {
  local work=$1
  install_binary "${work}/previous/artifact/bin/ty" "${work}/prefix"
  check_installed "${work}/previous/artifact" "${work}/prefix"
}

step_sample_app() {
  local work=$1
  local logs=$2
  write_sample_app "${work}/prefix" "${work}/project"
  serve_and_check "${work}/prefix" "${work}/project" "${logs}" previous
  commit_sample_app "${work}/project"
  fingerprint "${work}/project" > "${work}/project.sums"
  kept_fingerprint "${work}/project" > "${work}/kept.sums"
  cp "${work}/project.sums" "${work}/kept.sums" "${logs}/"
  echo "    recorded $(wc -l < "${work}/project.sums" | tr -d ' ') project files"
}

check_kept_unchanged() {
  local project=$1
  local expected=$2
  local label=$3
  local actual
  actual="$(kept_fingerprint "${project}")"
  if [ "${actual}" != "$(cat "${expected}")" ]; then
    diff <(cat "${expected}") <(printf '%s\n' "${actual}") >&2 || true
    fail "${label} changed the app's data or config"
  fi
  echo "    ok   server/data/notes.json, .env and package.json are byte-identical after ${label}"
}

step_upgrade() {
  local work=$1
  local logs=$2
  install_binary "${work}/candidate/artifact/bin/ty" "${work}/prefix"
  check_installed "${work}/candidate/artifact" "${work}/prefix"
  check_project_unchanged "${work}/project" "${work}/project.sums" "the upgrade"
  migrate_sample_app "${work}/prefix" "${work}/project"
  check_kept_unchanged "${work}/project" "${work}/kept.sums" "the migration"
  serve_and_check "${work}/prefix" "${work}/project" "${logs}" candidate
  check_kept_unchanged "${work}/project" "${work}/kept.sums" "running the candidate"
}

step_rollback() {
  local work=$1
  local logs=$2
  local version=$3
  rm -rf "${work}/previous"
  fetch_previous "${version}" "${work}"
  install_binary "${work}/previous/artifact/bin/ty" "${work}/prefix"
  check_installed "${work}/previous/artifact" "${work}/prefix"
  revert_sample_app "${work}/project"
  check_project_unchanged "${work}/project" "${work}/project.sums" "the rollback"
  serve_and_check "${work}/prefix" "${work}/project" "${logs}" rollback
  check_project_unchanged "${work}/project" "${work}/project.sums" "running the rolled-back release"
}

step_uninstall() {
  local work=$1
  rm -f "${work}/prefix/bin/ty"
  [ ! -e "${work}/prefix/bin/ty" ] || fail "the binary survived uninstall"
  local remaining
  remaining="$(find "${work}/prefix" -type f | wc -l | tr -d ' ')"
  [ "${remaining}" = "0" ] || fail "uninstall left ${remaining} file(s) behind"
  echo "    ok   uninstall left nothing behind"
}

drill() {
  local from=$1
  local candidate=$2
  local logs=$3
  command -v node > /dev/null 2>&1 || fail "the sample app's Yon handler needs node on PATH"
  command -v git > /dev/null 2>&1 || fail "the drill reverts the sample app's migration with git"

  local work
  work="$(mktemp -d "${TMPDIR:-/tmp}/ty-lifecycle-XXXXXX")"
  trap 'rm -rf "'"${work}"'"' EXIT
  if [ -z "${from}" ]; then
    from="$(latest_release)"
    touch "${work}/from-latest"
  fi
  export TAC_CACHE_DIR="${work}/cache"
  echo "lifecycle drill: ${from} -> candidate -> ${from} on $(platform_arch)-$(platform_os)"
  echo "log: ${logs}/lifecycle-drill.log"

  run_step fetch-previous fetch_previous "${from}" "${work}"
  if [ -z "${candidate}" ]; then
    run_step build-candidate build_candidate "${work}"
    candidate="${work}/candidate.tar.gz"
  fi
  run_step verify-candidate unpack_candidate "${candidate}" "${work}"
  run_step install-previous step_install_previous "${work}"
  run_step sample-app step_sample_app "${work}" "${logs}"
  run_step upgrade step_upgrade "${work}" "${logs}"
  run_step rollback step_rollback "${work}" "${logs}" "${from}"
  run_step uninstall step_uninstall "${work}"

  echo "PASS: release lifecycle drill ${from} -> candidate -> ${from}"
}

main() {
  local from="${TAC_DRILL_FROM:-}"
  local candidate=""
  local logs=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --from) from=$2; shift 2 ;;
      --candidate) candidate="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
      --log-dir) logs=$2; shift 2 ;;
      -h | --help) usage; return 0 ;;
      *) usage >&2; fail "unknown argument $1" ;;
    esac
  done
  [ -n "${logs}" ] || logs="${LOG_ROOT:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}}/TACHYON/lifecycle-drill-$(date -u +%Y%m%d-%H%M%S)"
  mkdir -p "${logs}"
  logs="$(cd "${logs}" && pwd)"
  drill "${from}" "${candidate}" "${logs}" 2>&1 | tee -a "${logs}/lifecycle-drill.log"
}

main "$@"
