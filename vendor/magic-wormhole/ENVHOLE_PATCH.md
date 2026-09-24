# EnvHole patch to magic-wormhole 0.8.1

Source: https://crates.io/crates/magic-wormhole/0.8.1
Upstream: https://github.com/magic-wormhole/magic-wormhole.rs
License: EUPL-1.2 (preserved in `LICENSE`)

EnvHole vendors the published 0.8.1 crate because its transit decoder allocates
a peer-declared 32-bit frame length before authentication and before the
application's bounded sink sees any bytes.

Local changes are intentionally narrow:

- `src/transit/transport.rs` rejects transit records above 1 MiB before
  allocation and contains a regression test for a hostile 2 MiB frame header.
- `src/lib.rs` suppresses deprecation warnings caused by the newer `time` crate
  API selected by the current lockfile; no behavior changes.
- Registry checksum metadata is omitted because this is a patched path
  dependency.

Remove the path patch when an upstream release enforces an equivalent
pre-allocation record limit. Do not update the vendored crate without reviewing
and reapplying or retiring this patch.
