#!/bin/sh
set -eu

REPOSITORY=Bots/envhole
VERSION=${ENVHOLE_VERSION:-}
INSTALL_DIR=${ENVHOLE_INSTALL_DIR:-${HOME:?HOME is not set}/.local/bin}
DOWNLOAD_DIR=
DESTINATION_TMP=

fail() {
  printf 'envhole installer: %s\n' "$*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

valid_version() {
  candidate=${1#v}
  [ "$candidate" != "$1" ] || return 1
  major=${candidate%%.*}
  rest=${candidate#*.}
  [ "$rest" != "$candidate" ] || return 1
  minor=${rest%%.*}
  patch=${rest#*.}
  [ "$patch" != "$rest" ] || return 1
  case "$patch" in
    *.*) return 1 ;;
  esac
  for component in "$major" "$minor" "$patch"; do
    [ -n "$component" ] || return 1
    case "$component" in
      *[!0-9]*) return 1 ;;
    esac
  done
}

cleanup() {
  if [ -n "$DESTINATION_TMP" ]; then
    rm -f -- "$DESTINATION_TMP"
  fi
  if [ -n "$DOWNLOAD_DIR" ]; then
    rm -rf -- "$DOWNLOAD_DIR"
  fi
}
trap cleanup 0
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

for command_name in curl tar sha256sum mktemp chmod mkdir mv uname wc; do
  need "$command_name"
done

[ "$(uname -s)" = Linux ] || fail "prebuilt releases currently support Linux only"
case "$(uname -m)" in
  x86_64|amd64) TARGET=x86_64-unknown-linux-musl ;;
  aarch64|arm64) TARGET=aarch64-unknown-linux-musl ;;
  *) fail "unsupported CPU architecture: $(uname -m)" ;;
esac

if [ -z "$VERSION" ]; then
  latest_url=$(curl \
    --proto '=https' \
    --proto-redir '=https' \
    --tlsv1.2 \
    --fail \
    --silent \
    --show-error \
    --location \
    --output /dev/null \
    --write-out '%{url_effective}' \
    "https://github.com/$REPOSITORY/releases/latest") ||
    fail "could not resolve the latest release"
  VERSION=${latest_url##*/}
fi
valid_version "$VERSION" || fail "release version must have the form vMAJOR.MINOR.PATCH"

PACKAGE=envhole-$VERSION-$TARGET
ARCHIVE=$PACKAGE.tar.gz
CHECKSUM=$ARCHIVE.sha256
BASE_URL=https://github.com/$REPOSITORY/releases/download/$VERSION

umask 077
DOWNLOAD_DIR=$(mktemp -d "${TMPDIR:-/tmp}/envhole-install.XXXXXX")

printf 'Downloading EnvHole %s for %s...\n' "$VERSION" "$TARGET"
for asset in "$ARCHIVE" "$CHECKSUM"; do
  curl \
    --proto '=https' \
    --proto-redir '=https' \
    --tlsv1.2 \
    --fail \
    --silent \
    --show-error \
    --location \
    --output "$DOWNLOAD_DIR/$asset" \
    "$BASE_URL/$asset" || fail "could not download $asset"
done

checksum_lines=$(wc -l < "$DOWNLOAD_DIR/$CHECKSUM")
checksum_bytes=$(wc -c < "$DOWNLOAD_DIR/$CHECKSUM")
[ "$checksum_lines" -eq 1 ] && [ "$checksum_bytes" -le 256 ] ||
  fail "release checksum file was malformed"
IFS=' ' read -r expected_digest expected_file unexpected < "$DOWNLOAD_DIR/$CHECKSUM" ||
  fail "release checksum file was malformed"
[ -n "$expected_digest" ] && [ "$expected_file" = "$ARCHIVE" ] &&
  [ -z "$unexpected" ] || fail "release checksum file named an unexpected asset"
[ "${#expected_digest}" -eq 64 ] || fail "release checksum was not SHA-256"
case "$expected_digest" in
  *[!0-9a-fA-F]*) fail "release checksum was not SHA-256" ;;
esac
(
  cd "$DOWNLOAD_DIR"
  sha256sum -c "$CHECKSUM"
) || fail "release checksum verification failed"

mkdir -p -- "$INSTALL_DIR"
[ ! -d "$INSTALL_DIR/envhole" ] ||
  fail "installation target is a directory or points to one: $INSTALL_DIR/envhole"
DESTINATION_TMP=$(mktemp "$INSTALL_DIR/.envhole.XXXXXX")
tar -xOzf "$DOWNLOAD_DIR/$ARCHIVE" "$PACKAGE/envhole" > "$DESTINATION_TMP" ||
  fail "release archive did not contain the expected binary"
chmod 0755 "$DESTINATION_TMP"

actual_version=$("$DESTINATION_TMP" --version) || fail "downloaded binary did not run"
[ "$actual_version" = "envhole ${VERSION#v}" ] ||
  fail "downloaded binary reported an unexpected version: $actual_version"

[ ! -d "$INSTALL_DIR/envhole" ] ||
  fail "installation target became a directory: $INSTALL_DIR/envhole"
mv -fT -- "$DESTINATION_TMP" "$INSTALL_DIR/envhole" ||
  fail "could not replace $INSTALL_DIR/envhole"
DESTINATION_TMP=

printf 'Installed EnvHole %s to %s/envhole\n' "$VERSION" "$INSTALL_DIR"
case ":${PATH:-}:" in
  *":$INSTALL_DIR:"*) ;;
  *) printf 'Add %s to PATH to run envhole from anywhere.\n' "$INSTALL_DIR" ;;
esac
