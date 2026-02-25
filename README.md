# Emerald Password Share

A secure, self-destructing note and file sharing service built for Emerald Group. This project is a fork of [cryptgeon](https://github.com/cupcakearmy/cryptgeon), customised to meet Emerald's internal branding and requirements.

---

## How It Works

Each note is assigned a randomly generated **256-bit ID** and a separate **256-bit encryption key**. The ID is used to store and retrieve the note on the server, while the key never leaves the client.

Before the note is sent to the server, it is encrypted in the browser using **AES-GCM** with the 256-bit key. The server stores only the encrypted ciphertext in Redis (entirely in memory — nothing is ever written to disk). Because the encryption key is embedded in the URL fragment (`#key`) and never transmitted to the server, the server is cryptographically incapable of reading note contents.

When a recipient opens the link, their browser fetches the encrypted blob, extracts the key from the URL fragment, and decrypts the note locally.

Notes self-destruct after either a set number of views or a time limit — whichever comes first. Once a note is destroyed, it is gone permanently.

A **"Send via email"** button is available after creating a note, pre-addressed to `support@emerald-group.co.uk` with the secure link ready to send.

---

## Building the Docker Image

### Prerequisites

- Docker and Docker Compose installed
- No other dependencies needed — everything builds inside Docker

### Build

```bash
docker build -t emerald-cryptgeon:latest .
```

The Dockerfile uses a multi-stage build:

1. **Frontend** — Node 22 Alpine with pnpm builds the SvelteKit frontend into a static bundle.
2. **Backend** — Rust 1.85 Alpine compiles the Axum-based API server.
3. **Runner** — A minimal Alpine image assembles the final image from the two build stages.

### Run with Docker Compose

A `docker-compose.yaml` is included at the root of the project:

```bash
docker compose up -d
```

This starts:
- A **Redis** instance (in-memory only, no persistence) for note storage
- The **app** on port `8001` (mapped from internal port `8000`)

To rebuild the app image before starting:

```bash
docker compose up --build
```

---

## Attribution

This project is a fork of [cryptgeon](https://github.com/cupcakearmy/cryptgeon) by [cupcakearmy](https://github.com/cupcakearmy), modified for internal use by Emerald Group. The original project is open source and the fork retains its licence.
