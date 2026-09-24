# Security policy

EnvHole 0.1 is an initial implementation, not an independently audited product.
Its cryptography and wire protocol are provided by magic-wormhole; EnvHole does
not implement PAKE, encryption, authentication primitives, or key derivation.

Do not put real secrets in bug reports, screenshots, CI logs, or test fixtures.
Report vulnerabilities privately using the hosting repository's private
vulnerability reporting feature when available. If no private reporting route
is configured, ask the maintainer for one without publishing exploit details
or affected secrets. No dedicated security mailbox is configured in this repo.

Use only trusted endpoints and a destination directory whose entries and
ancestors cannot be modified by an adversary. Share codes through a trusted
channel. Anyone with the code may connect; there is no account identity check.
No claim of protection from malware, administrators, terminal capture, swap,
core dumps, backups, or compromised dependencies is made.

Payload values are never deliberately logged. Only validated ASCII names,
counts, fixed errors, and the sender's code are displayed. No tracing/logging
subscriber is installed, and peer error strings and filenames are not printed.
The payload exists in memory, then in the explicitly approved output and briefly
in a mode-0600 temporary file during atomic saving. Forced termination can leave
that temporary file. No secure erasure or memory-locking guarantee is provided.

Run cargo audit regularly. CI checks current RustSec advisories against the
lockfile; upstream advisories and service behavior can change after release.
See docs/threat-model.md for limitations.
