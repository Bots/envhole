# Protocol

EnvHole 0.1 uses a vendored magic-wormhole 0.8.1 with the crate's standard
`transfer::APP_CONFIG`, default rendezvous, default transit relay hint, and
`transit::Abilities::ALL`. Direct transit may be selected when reachable;
otherwise the public transit relay is available. It uses the stable v1 file
transfer API, not experimental v2 or an EnvHole-specific encryption format.
The vendored copy adds a pre-allocation 1 MiB ceiling to the peer-declared
transit-record length and includes a hostile-frame regression test. No
cryptographic code is changed.

1. Sender reads at most 1 MiB plus one sentinel byte, validates metadata,
   displays only names/count, and obtains approval.
2. `MailboxConnection::create(APP_CONFIG, 2)` allocates a nameplate and two-word
   password. The sender prints the resulting short code.
3. Receiver connects to that code without allocating a new nameplate.
   `Wormhole::connect` on each side performs the crate's protocol handshake.
4. Sender calls `transfer::send_file` with constant filename `envhole.env`,
   exact byte count, and a cursor over the original bytes. No original path
   or separate manifest is sent.
5. Receiver calls `transfer::request_file`, rejects oversized offers, and
   accepts into an in-memory AsyncWrite sink capped at the advertised length
   (itself at most 1 MiB). The crate handles transit encryption and integrity.
6. Receiver checks actual length, parses metadata, displays names/count, and
   obtains approval before publishing from a private same-directory temporary
   file. Forced replacement uses a rename operation; no-clobber publication is
   not guaranteed atomic on every platform. The transfer acknowledgement
   precedes user approval; sender success does not imply an output was saved.

No custom PAKE, AEAD, key derivation, checksum protocol, shell evaluation,
or serialization of secret values is introduced. Input bytes are unchanged.

References:
* [Crate API](https://docs.rs/magic-wormhole/0.8.1/magic_wormhole/)
* [Upstream implementation](https://github.com/magic-wormhole/magic-wormhole.rs)
* [File transfer protocol](https://github.com/magic-wormhole/magic-wormhole-protocols/blob/main/file-transfer-protocol.md)
