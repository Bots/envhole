# Self-hosted Magic Wormhole services

This Compose stack runs the two infrastructure services used by EnvHole:

- a rendezvous/mailbox server on TCP 4000 (WebSocket path `/v1`);
- a transit relay on TCP 4001.

It does not store transferred `.env` payloads. The mailbox stores encrypted
control messages; bulk traffic is direct when possible or forwarded as opaque
encrypted bytes by the transit relay.

The image builds for the Raspberry Pi architectures provided by the official
multi-platform Python image, including `linux/arm64` and `linux/arm/v7`.
Server packages and the base-image digest are pinned.

## Raspberry Pi

Install Docker Engine with the Compose plugin, clone EnvHole, then:

    cd envhole/deploy/server
    cp .env.example .env
    docker compose up -d --build
    docker compose ps

For LAN or VPN testing, replace `PI_HOST` with the Pi's LAN address or private
DNS name on both clients:

    export ENVHOLE_RENDEZVOUS_URL=ws://PI_HOST:4000/v1
    export ENVHOLE_TRANSIT_RELAY=tcp://PI_HOST:4001

Then use the normal `envhole send` and `envhole receive` commands. Both peers
must use the same rendezvous URL. Each peer advertises its relay hint, but using
the same relay setting makes behavior deterministic when direct connectivity
fails.

## Exposure

For access beyond a trusted LAN, prefer a private VPN such as WireGuard or
Tailscale. If publishing on the internet:

- forward TCP 4001 to the Pi for the transit relay;
- terminate TLS in a reverse proxy and expose the mailbox as
  `wss://your-name.example/v1` rather than raw `ws://`;
- set `MAILBOX_BIND=127.0.0.1` when the reverse proxy is on the Pi;
- keep the relay's TCP port reachable by both clients.

Magic Wormhole encrypts and authenticates payloads end to end, but TLS still
protects connection metadata and prevents local network manipulation of the
WebSocket transport.

## Operations

    docker compose logs -f
    docker compose pull                 # base images are build inputs, not services
    docker compose build --pull
    docker compose up -d
    docker compose down                 # keeps named volumes
    docker compose down --volumes       # deletes mailbox and usage databases

The services run as UID/GID 65532, with all Linux capabilities dropped, a
read-only root filesystem, bounded temporary storage, and persistent named
volumes for SQLite databases.
