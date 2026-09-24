# Threat model

## Assets and trust boundaries

The assets are payload bytes and the one-time code. Variable names, count,
timing, IP addresses, transfer size, and the fact of communication are metadata.
Names are intentionally shown on both endpoints.

Trust the sending and receiving machines, their OS and dependencies, the code
delivery channel, and the destination directory and its ancestors. The
rendezvous and transit services need not be trusted with plaintext; protocol
authentication and encryption are delegated entirely to magic-wormhole.
Services can still deny service or observe connection metadata.

## Protections

Magic Wormhole uses its password-authenticated exchange to establish an
authenticated encrypted channel, then encrypted transit for the payload.
EnvHole uses the maintained crate rather than implementing cryptographic steps.
It does not treat possession of the code as a verified human identity.

An unauthenticated/incorrect peer cannot normally decrypt a successful transfer.
A peer with the code can receive or supply content. EnvHole treats received
content and protocol metadata as untrusted: it ignores offered paths/names,
bounds the advertised and written payload sizes, validates UTF-8 and keys,
refuses duplicates, never evaluates interpolation, and requires save approval
unless the user explicitly gives --yes. No plaintext is persisted before that
approval.

Output creation uses tempfile and a no-clobber publish by default. Forced
replacement uses the platform rename operation, never opens the target to
truncate it, and rejects symlinks before staging and immediately before
publication. No-clobber publication is not guaranteed atomic on every platform.
The file is synced before publication and has mode 0600 on Unix.

## Limitations

An attacker modifying directory entries between checks can race forced
replacement. An attacker controlling parent directories can redirect path
resolution. Use a trusted directory; this is not a descriptor-relative hardened
filesystem sandbox. No-clobber publication prevents unintended replacement by
default. Atomic visibility is distinct from durability across power loss.

The vendored magic-wormhole patch rejects peer-declared transit records above
1 MiB before allocation. The cap still does not cover every allocation in the
networking library or total process memory. Malicious peers can stall
connections or consume resources elsewhere in protocol processing. There is no
application-wide timeout, rate limiter, or hardened hostile-server sandbox.

The public rendezvous uses the crate's default URL (currently ws, not wss).
Payload confidentiality/authentication relies on Magic Wormhole's end-to-end
protocol, not transport TLS. Network metadata is not hidden.

Local malware, privileged users, process-memory inspection, swap/core dumps,
terminal logging, clipboard history, shell history of receive codes, filesystem
snapshots, and backups are out of scope. Secret memory is not guaranteed erased.
Crashes during approved saving may leave a private temporary file. Dependency
or protocol vulnerabilities remain possible; this application has not been
independently audited.
