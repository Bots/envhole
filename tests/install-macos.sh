#!/bin/sh
set -eu

[ "$(uname -s)" = Darwin ] || {
  printf '%s\n' 'macOS installer test must run on macOS' >&2
  exit 1
}

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/envhole-macos-installer-test.XXXXXX")
trap 'rm -rf "$TMP_ROOT"' 0
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

case "$(uname -m)" in
  x86_64|amd64) TARGET=x86_64-apple-darwin ;;
  arm64|aarch64) TARGET=aarch64-apple-darwin ;;
  *) printf '%s\n' 'unsupported test architecture' >&2; exit 1 ;;
esac

FIXTURES=$TMP_ROOT/fixtures
MOCK_BIN=$TMP_ROOT/mock-bin
INSTALL_DIR=$TMP_ROOT/'install dir'/bin
PACKAGE=envhole-v0.1.0-$TARGET
ARCHIVE=$PACKAGE.tar.gz
CHECKSUM=$ARCHIVE.sha256
mkdir -p "$FIXTURES/$PACKAGE" "$MOCK_BIN" "$INSTALL_DIR"

cat > "$FIXTURES/$PACKAGE/envhole" <<'SCRIPT'
#!/bin/sh
printf '%s\n' 'envhole 0.1.0'
SCRIPT
chmod 0755 "$FIXTURES/$PACKAGE/envhole"
tar -C "$FIXTURES" -czf "$FIXTURES/$ARCHIVE" "$PACKAGE"
(
  cd "$FIXTURES"
  shasum -a 256 "$ARCHIVE" > "$CHECKSUM"
)

cat > "$MOCK_BIN/curl" <<'SCRIPT'
#!/bin/sh
set -eu
output=
url=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) output=$2; shift 2 ;;
    --proto|--proto-redir) [ "${2:-}" = '=https' ]; shift 2 ;;
    --write-out) shift 2 ;;
    --tlsv1.2|--fail|--silent|--show-error|--location) shift ;;
    *) url=$1; shift ;;
  esac
done
[ -n "$output" ] && [ -n "$url" ]
cp "$ENVHOLE_TEST_FIXTURES/${url##*/}" "$output"
SCRIPT
chmod 0755 "$MOCK_BIN/curl"

PATH="$MOCK_BIN:$PATH" \
ENVHOLE_TEST_FIXTURES="$FIXTURES" \
ENVHOLE_VERSION=v0.1.0 \
ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
sh "$ROOT/install.sh" >/dev/null
[ "$("$INSTALL_DIR/envhole" --version)" = 'envhole 0.1.0' ]

printf '%s\n' old-version > "$INSTALL_DIR/envhole"
PATH="$MOCK_BIN:$PATH" \
ENVHOLE_TEST_FIXTURES="$FIXTURES" \
ENVHOLE_VERSION=v0.1.0 \
ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
sh "$ROOT/install.sh" >/dev/null
[ "$("$INSTALL_DIR/envhole" --version)" = 'envhole 0.1.0' ]

printf '%s\n' 'macOS installer tests passed'
