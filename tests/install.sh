#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/envhole-installer-test.XXXXXX")
trap 'rm -rf "$TMP_ROOT"' EXIT HUP INT TERM

FIXTURES=$TMP_ROOT/fixtures
MOCK_BIN=$TMP_ROOT/mock-bin
INSTALL_DIR=$TMP_ROOT/'install dir'/bin
PACKAGE=envhole-v0.1.0-x86_64-unknown-linux-musl
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
  sha256sum "$ARCHIVE" > "$CHECKSUM"
)

cat > "$MOCK_BIN/uname" <<'SCRIPT'
#!/bin/sh
case "${1:-}" in
  -s) printf '%s\n' Linux ;;
  -m) printf '%s\n' x86_64 ;;
  *) exit 2 ;;
esac
SCRIPT

cat > "$MOCK_BIN/curl" <<'SCRIPT'
#!/bin/sh
set -eu
output=
url=
write_out=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output)
      output=$2
      shift 2
      ;;
    --proto|--proto-redir)
      [ "${2:-}" = '=https' ]
      shift 2
      ;;
    --write-out)
      write_out=$2
      shift 2
      ;;
    --tlsv1.2|--fail|--silent|--show-error|--location)
      shift
      ;;
    *) url=$1; shift ;;
  esac
done
[ -n "$url" ]
case "$url" in
  */releases/latest)
    [ "$write_out" = '%{url_effective}' ]
    printf '%s' 'https://github.com/Bots/envhole/releases/tag/v0.1.0'
    exit 0
    ;;
esac
[ -n "$output" ]
asset=${url##*/}
if [ "${ENVHOLE_TEST_FAIL_ASSET:-}" = "$asset" ]; then
  printf '%s\n' partial-download > "$output"
  exit 22
fi
cp "$ENVHOLE_TEST_FIXTURES/$asset" "$output"
SCRIPT
chmod 0755 "$MOCK_BIN/uname" "$MOCK_BIN/curl"

PATH="$MOCK_BIN:$PATH" \
ENVHOLE_TEST_FIXTURES="$FIXTURES" \
ENVHOLE_VERSION=v0.1.0 \
ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
sh "$ROOT/install.sh" >/dev/null

[ -x "$INSTALL_DIR/envhole" ]
[ "$("$INSTALL_DIR/envhole" --version)" = 'envhole 0.1.0' ]

rm -f "$INSTALL_DIR/envhole"
PATH="$MOCK_BIN:$PATH" \
ENVHOLE_TEST_FIXTURES="$FIXTURES" \
ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
sh "$ROOT/install.sh" >/dev/null
[ "$("$INSTALL_DIR/envhole" --version)" = 'envhole 0.1.0' ]

rm -f "$INSTALL_DIR/envhole"
mkdir "$INSTALL_DIR/envhole"
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted a directory as the destination' >&2
  exit 1
fi
[ -d "$INSTALL_DIR/envhole" ]
rm -rf "$INSTALL_DIR/envhole"

mkdir "$TMP_ROOT/symlink-target"
ln -s "$TMP_ROOT/symlink-target" "$INSTALL_DIR/envhole"
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted a directory symlink as the destination' >&2
  exit 1
fi
[ -L "$INSTALL_DIR/envhole" ]
rm -f "$INSTALL_DIR/envhole"

printf '%s\n' 'previous-version' > "$INSTALL_DIR/envhole"
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_TEST_FAIL_ASSET="$ARCHIVE" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted a truncated download' >&2
  exit 1
fi
[ "$(cat "$INSTALL_DIR/envhole")" = 'previous-version' ]

checksum_line=$(cd "$FIXTURES" && sha256sum "$ARCHIVE")
printf '%s\n%s\n' "$checksum_line" "$checksum_line" > "$FIXTURES/$CHECKSUM"
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted multiple checksum records' >&2
  exit 1
fi
[ "$(cat "$INSTALL_DIR/envhole")" = 'previous-version' ]

printf '%s\n' 'not-the-real-checksum  envhole.tar.gz' > "$FIXTURES/$CHECKSUM"
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted a bad checksum' >&2
  exit 1
fi
[ "$(cat "$INSTALL_DIR/envhole")" = 'previous-version' ]

checksum_line=$(cd "$FIXTURES" && sha256sum "$ARCHIVE")
printf '%s trailing-garbage\n' "$checksum_line" > "$FIXTURES/$CHECKSUM"
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted trailing checksum garbage' >&2
  exit 1
fi
[ "$(cat "$INSTALL_DIR/envhole")" = 'previous-version' ]

printf '%s\n' readme > "$FIXTURES/$PACKAGE/README.md"
tar -C "$FIXTURES" -czf "$FIXTURES/$ARCHIVE" "$PACKAGE/README.md"
(
  cd "$FIXTURES"
  sha256sum "$ARCHIVE" > "$CHECKSUM"
)
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted an archive without envhole' >&2
  exit 1
fi
[ "$(cat "$INSTALL_DIR/envhole")" = 'previous-version' ]

cat > "$FIXTURES/$PACKAGE/envhole" <<'SCRIPT'
#!/bin/sh
printf '%s\n' 'envhole 9.9.9'
SCRIPT
chmod 0755 "$FIXTURES/$PACKAGE/envhole"
tar -C "$FIXTURES" -czf "$FIXTURES/$ARCHIVE" "$PACKAGE/envhole"
(
  cd "$FIXTURES"
  sha256sum "$ARCHIVE" > "$CHECKSUM"
)
if PATH="$MOCK_BIN:$PATH" \
  ENVHOLE_TEST_FIXTURES="$FIXTURES" \
  ENVHOLE_VERSION=v0.1.0 \
  ENVHOLE_INSTALL_DIR="$INSTALL_DIR" \
  sh "$ROOT/install.sh" >/dev/null 2>&1; then
  printf '%s\n' 'installer accepted a wrong-version binary' >&2
  exit 1
fi
[ "$(cat "$INSTALL_DIR/envhole")" = 'previous-version' ]

printf '%s\n' 'installer tests passed'
