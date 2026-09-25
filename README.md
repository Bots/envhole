# EnvHole 0.1

A small Rust CLI for transferring one UTF-8 .env payload using
[magic-wormhole](https://docs.rs/magic-wormhole/0.8.1/magic_wormhole/).
No accounts, telemetry, analytics, secret database, or custom cryptography.

## Build

Install current stable Rust, then:

```sh
cargo build --release --locked
# binary: target/release/envhole
```

## Two terminals (or two machines)

Sender:

```sh
envhole send .env
# Displays variable names with *** and a count.
# Confirm with y. The result includes both the temporary code and a complete
# receiver command carrying the same rendezvous and transit configuration:
#
# Code: 7-example-words
# Receive command:
# ENVHOLE_RENDEZVOUS_URL='ws://server:4000/v1' ENVHOLE_TRANSIT_RELAY='tcp://server:4001' envhole receive '7-example-words' --output .env.received
```

Receiver:

```sh
envhole receive 7-example-words --output .env.received
# Receives into memory, displays names/count, asks before saving.
```

The example code is a placeholder; use the sender's generated code.
Treat the code as a temporary secret. It is printed to stdout for sharing.
Do not post it publicly. The generated command contains that code and may be
saved in the receiver's shell history; use it only on the intended receiver
and remove the history entry if that matters for your threat model. The command
uses POSIX-shell quoting (Bash, Zsh, Dash, and similar shells). The generated
`.env.received` destination still refuses an existing file unless the receiver
deliberately adds `--force`.

To use self-hosted infrastructure, configure both peers identically:

```sh
export ENVHOLE_RENDEZVOUS_URL=ws://PI_HOST:4000/v1
export ENVHOLE_TRANSIT_RELAY=tcp://PI_HOST:4001
envhole send .env
```

The equivalent global flags are `--rendezvous-url` and `--transit-relay`.
A Raspberry Pi Docker Compose deployment is provided in
[`deploy/server`](deploy/server/README.md).

For automation:

```sh
cat .env | envhole send - --yes
envhole receive 7-example-words --output .env.received --yes
# Add --force to replace an existing regular file.
```

Without --yes, only y/yes (case insensitive) approves; EOF declines.
Sending stdin requires --yes because stdin is the payload stream.
The sender's success means the peer received the bytes, **not** that the peer
approved saving them.

## Exact scope

* Transfers exact original bytes: no newline conversion, normalization, variable
  expansion, shell execution, or value rewriting.
* Maximum payload: 1 MiB (1,048,576 bytes), fixed in v0.1. Both reading and
  received data are bounded; oversized offers are rejected.
* Preview accepts blank lines, # comments, optional `export `, CRLF, unquoted
  values, single/double quoted values (including multiple lines), and escaped
  quotes inside double quotes. Trailing comments after quotes are accepted.
* Keys must match `[A-Za-z_][A-Za-z0-9_]*`; duplicates are rejected. Invalid
  UTF-8, forbidden control characters, unterminated quotes, and trailing junk
  after quoted values are rejected with errors that never quote the input.
  This is a metadata parser, not a complete shell or dotenv interpreter.
* Values remain opaque. Names themselves are visible and may reveal sensitive
  context. Do not encode secret values into variable names.
* Receives stay in memory until validated and approved. Saving uses a private
  temporary file in the destination directory and syncs it before publication.
  Forced replacement uses the platform rename operation. No-clobber publication
  prevents overwrite but is not guaranteed atomic on every platform. Unix file
  mode is 0600, including replacements.
* Existing targets require --force. Symlinks (including dangling links) and
  non-regular targets are refused. Use a trusted destination directory;
  protection against an attacker replacing parent directories is out of scope.
* No folders, resume, stored history, or guaranteed
  cancellation cleanup on forced termination. Ctrl-C exits. A crash while
  saving may leave a private temporary file. Memory is not guaranteed zeroized.
* Unix permissions are tested on Linux. Windows ACL guarantees are not provided.
  Parent-directory fsync/power-loss durability is not promised.
* Uses a vendored magic-wormhole 0.8.1 with one narrow hardening patch: transit
  records larger than 1 MiB are rejected before allocation. The upstream crate
  otherwise trusts a peer-controlled 32-bit record length. The patch has its own
  regression test and should be removed after an equivalent upstream release.
* Uses the standard file-transfer application ID and defaults to public
  infrastructure. Rendezvous and transit URLs can be replaced together for a
  self-hosted deployment. Compatible peers can offer arbitrary files; EnvHole
  validates before saving. Connections may wait until interrupted if a peer
  does not complete the handshake.

See [security policy](SECURITY.md), [threat model](docs/threat-model.md),
and [protocol](docs/protocol.md).

## Validation

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo test --manifest-path vendor/magic-wormhole/Cargo.toml \
  transit::transport::tests::oversized_transit_record_is_rejected_before_body_read -- --exact
cargo audit
# Network smoke test, synthetic payload only, bounded process waits:
cargo test --test network -- --ignored --nocapture
```

The network test launches two actual envhole processes against the configured
infrastructure, checks masked output,
and compares original and saved bytes. It is ignored by default so offline CI
does not rely on a public service. CI starts the Compose services and runs this
test against them. It was also exercised successfully against the public
service during v0.1 development. Local tests cover parsing, limits, previews,
CLI validation, confirmation refusal, symlink refusal, replacement, and Unix
permissions. The suite currently contains 15 non-network tests plus the
opt-in network smoke test.
