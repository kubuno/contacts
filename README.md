<!--
  SPDX-FileCopyrightText: 2026 Kubuno contributors
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<p align="center">
  <img src=".github/logo.png" alt="Kubuno Contacts logo" width="128" height="128">
</p>

# Kubuno Contacts

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-edition_2021-orange.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)
![Module](https://img.shields.io/badge/Kubuno-module-4D38DB.svg)

**Kubuno Contacts — address book, groups & labels, CardDAV, instance directory**

A module for [Kubuno](https://github.com/kubuno/core), the self-hosted, libre (AGPLv3) cloud platform.

## Features

- **Full-featured address book** — rich contact records (names, pronouns, organization & job title, emails, phones, postal addresses, URLs, dates, relations, instant messaging handles, custom fields, notes, avatars), starring, archiving, blocking, and a trash with restore.
- **Groups & labels** — organize contacts into colored groups and freeform labels, both managed straight from the sidebar. Every view (all, starred, a group, a label, smart views…) has its own shareable URL, so direct links and the browser Back button just work.
- **Smart views** — birthdays, frequently contacted, follow-up suggestions, and a duplicate finder with one-click merge (or ignore).
- **Reminders** — birthday and custom follow-up reminders with recurrence, surfaced as a due-count badge in the sidebar.
- **Import & export** — vCard (`.vcf`) and CSV import; vCard and CSV export of your whole address book.
- **CardDAV** — sync your contacts with phones and desktop clients via the built-in CardDAV endpoint (per-user tokens).
- **Instance directory** — browse the other users of your Kubuno instance and add them to your contacts in one click.
- **Sharing** — share a contact with other users of the instance, or through a public link.
- **Cross-module integration** — other Kubuno modules can open a globally-mounted **contact picker** (published on the core's service registry, with graceful degradation when Contacts is not installed), and a contact copied with "Copy for Kubuno" pastes as a **rich contact card** in consumer modules such as Chat or Notes. Contacts also plugs into the core's cross-module labels.
- **@mention provider** — Contacts publishes a mention provider on the core's extension registry, so typing `@` in any mention-enabled text field across the platform (a mail body, a comment, a rich-text editor…) suggests your contacts with avatar and email and inserts them as a removable chip — with the same graceful degradation when Contacts is absent.
- **Delta sync** — cursor-based `/delta` endpoints (contacts, labels, groups, reminders) with monotonic change sequences and tombstones, powering incremental pulls by local-first clients; client-minted IDs are honoured on create for offline replay.
- **Per-user settings** — each user tunes the module from a dedicated settings page.

## Architecture

A standalone Rust process that registers with the [core](https://github.com/kubuno/core) at startup; the core proxies its routes (`/api/v1/contacts/*`) and serves its runtime-loaded React frontend bundle.

- **Backend** — `src/`: Axum + SQLx (PostgreSQL, schema `contacts`); migrations in `migrations/`.
- **Frontend** — `frontend/`: a React bundle built to `entry.js`, consuming `@kubuno/sdk`, `@kubuno/ui` and `@kubuno/drive` from npm (provided by the host at runtime via the import map).

## Install

This module ships in the **all-in-one [Kubuno](https://github.com/kubuno/core) Docker image** (`ghcr.io/kubuno/kubuno`) — the easiest way to self-host a full Kubuno instance (core + every module). See **[kubuno/docker](https://github.com/kubuno/docker)** for `docker compose` instructions.

**Native packages** for every supported platform are attached to each [GitHub Release](https://github.com/kubuno/contacts/releases): Debian/Ubuntu (`.deb`), Fedora/RHEL/openSUSE (`.rpm`), Windows (NSIS installer), and macOS (`.pkg`). They all install the module into an existing Kubuno core installation and restart the service.

To build this module from source, see below.

## Build

**Requirements:** Rust ≥ 1.82, Node.js ≥ 24, PostgreSQL 16.

```bash
cargo build --release                     # → target/release/kubuno-contacts
cd frontend && npm ci && npm run build     # → dist/{entry.js, entry.css}
bash build_deb.sh                          # → dist/kubuno-contacts_*.deb
```

Native packages for the other platforms are produced by self-detecting scripts sharing the same layout as the `.deb` (also run by CI on release tags):

```bash
bash build_rpm.sh                          # → dist/kubuno-contacts-*.rpm   (Fedora/RHEL/openSUSE)
bash build_windows.sh                      # → dist/kubuno-contacts-setup-*-x64.exe (NSIS; cargo-xwin from Linux)
bash build_macos.sh                        # → dist/kubuno-contacts-*-arm64.pkg     (run on a Mac)
```

> Shared dependencies come from Kubuno — no `kubuno/core` checkout required:
> - **Rust** — shared crates via tagged git dependencies on `kubuno/core`.
> - **Frontend** — `@kubuno/sdk`, `@kubuno/ui`, `@kubuno/drive` from the `@kubuno` npm scope.

## License

[AGPL-3.0-or-later](LICENSE) © Kubuno contributors.
