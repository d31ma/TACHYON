#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  echo "FAIL: $*" >&2
  exit 1
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
    (cd "$1" && shasum -a 256 --status --check SHA256SUMS)
  fi
}

build_artifact() {
  "${REPO_ROOT}/scripts/release/build-artifact.sh" "$1" > /dev/null
}

artifact_directory() {
  find "$1" -maxdepth 1 -type d -name 'tachyon-*' | head -1
}

release_archive() {
  find "$1" -maxdepth 1 -type f -name 'tachyon-*.tar.gz' | head -1
}

check_same_binary() {
  local first=$1
  local second=$2
  if [ "$(checksum "${first}/bin/ty")" != "$(checksum "${second}/bin/ty")" ]; then
    fail "two builds of one commit produced different binaries"
  fi
  echo "    ok   the binary is bit-identical across builds"
}

check_same_archive() {
  local first_build=$1
  local second_build=$2
  local first_archive
  first_archive="$(release_archive "${first_build}")"
  local second_archive
  second_archive="$(release_archive "${second_build}")"
  [ -n "${first_archive}" ] || fail "the first release archive was not produced"
  [ -n "${second_archive}" ] || fail "the second release archive was not produced"
  if [ "$(checksum "${first_archive}")" != "$(checksum "${second_archive}")" ]; then
    fail "two builds of one commit produced different release archives"
  fi
  echo "    ok   the release archive is bit-identical across builds"
}

check_published_checksums() {
  local artifact=$1
  echo "==> verifying the artifact before trusting it"
  verify_checksums "${artifact}" || fail "published checksums did not verify"
  echo "    ok   every published file matches SHA256SUMS"
}

check_tamper_detection() {
  local artifact=$1
  local tampered=$2
  echo "==> tamper detection"
  cp -R "${artifact}" "${tampered}"
  printf '\0' >> "${tampered}/bin/ty"
  if verify_checksums "${tampered}" 2> /dev/null; then
    fail "a modified binary passed checksum verification"
  fi
  echo "    ok   a modified artifact fails verification"
}

install_release() {
  local artifact=$1
  local prefix=$2
  local expected_version=$3
  echo "==> install"
  mkdir -p "${prefix}/bin"
  install -m 0755 "${artifact}/bin/ty" "${prefix}/bin/ty"
  "${prefix}/bin/ty" --version > /dev/null || fail "the installed binary does not run"
  local installed
  installed="$("${prefix}/bin/ty" --version)"
  [ "${installed}" = "${expected_version}" ] \
    || fail "installed version '${installed}' does not match VERSION '${expected_version}'"
  echo "    ok   installed ${installed}"
}

check_installed_tool_builds() {
  local prefix=$1
  local project=$2
  echo "==> the installed tool builds a real project"
  mkdir -p "${project}/client/pages" "${project}/client/components/release/page"
  printf '<release-page />' > "${project}/client/pages/tac.html"
  printf '<main aria-label="Release"><h1>Release</h1></main>' > "${project}/client/components/release/page/tac.html"
  "${prefix}/bin/ty" build "${project}" > /dev/null || fail "the installed binary cannot build"
  [ -f "${project}/dist/index.html" ] || fail "the installed binary published nothing"
  echo "    ok   the installed tool built and published a project"
}

check_upgrade() {
  local artifact=$1
  local prefix=$2
  local project=$3
  local previous=$4
  echo "==> upgrade"
  cp "${prefix}/bin/ty" "${previous}"
  install -m 0755 "${artifact}/bin/ty" "${prefix}/bin/ty"
  "${prefix}/bin/ty" --version > /dev/null || fail "the upgraded binary does not run"
  "${prefix}/bin/ty" build "${project}" > /dev/null || fail "the upgraded binary cannot build"
  echo "    ok   upgraded in place and still builds"
}

check_rollback() {
  local previous=$1
  local prefix=$2
  local project=$3
  local installed=$4
  echo "==> rollback"
  install -m 0755 "${previous}" "${prefix}/bin/ty"
  "${prefix}/bin/ty" --version > /dev/null || fail "the rolled-back binary does not run"
  [ "$("${prefix}/bin/ty" --version)" = "${installed}" ] || fail "rollback produced a different version"
  "${prefix}/bin/ty" build "${project}" > /dev/null || fail "the rolled-back binary cannot build"
  echo "    ok   rolled back to ${installed} and still builds"
}

check_uninstall() {
  local prefix=$1
  echo "==> uninstall"
  rm -f "${prefix}/bin/ty"
  [ ! -e "${prefix}/bin/ty" ] || fail "the binary survived uninstall"
  local remaining
  remaining="$(find "${prefix}" -type f | wc -l | tr -d ' ')"
  [ "${remaining}" = "0" ] || fail "uninstall left ${remaining} file(s) behind"
  echo "    ok   uninstall left nothing behind"
}

main() {
  local expected_version
  expected_version="$(tr -d '[:space:]' < "${REPO_ROOT}/VERSION")"
  WORK="$(mktemp -d "${TMPDIR:-/tmp}/ty-lifecycle-XXXXXX")"
  trap 'rm -rf "${WORK}"' EXIT
  local prefix="${WORK}/prefix"
  local project="${WORK}/project"
  local previous="${WORK}/ty.previous"

  echo "==> building two artifacts to exercise an upgrade and a rollback"
  build_artifact "${WORK}/build-a"
  local first
  first="$(artifact_directory "${WORK}/build-a")"
  [ -n "${first}" ] || fail "the first artifact was not produced"

  echo "==> reproducibility: the same source must produce the same binary"
  build_artifact "${WORK}/build-b"
  local second
  second="$(artifact_directory "${WORK}/build-b")"
  check_same_binary "${first}" "${second}"
  check_same_archive "${WORK}/build-a" "${WORK}/build-b"

  check_published_checksums "${first}"
  check_tamper_detection "${first}" "${WORK}/tamper"
  install_release "${first}" "${prefix}" "${expected_version}"
  check_installed_tool_builds "${prefix}" "${project}"
  check_upgrade "${second}" "${prefix}" "${project}" "${previous}"
  check_rollback "${previous}" "${prefix}" "${project}" "${expected_version}"
  check_uninstall "${prefix}"

  echo "PASS: release lifecycle drill"
}

main "$@"
