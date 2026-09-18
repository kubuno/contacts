<!--
  SPDX-FileCopyrightText: 2026 Kubuno contributors
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<div align="center">

<img src=".github/logo.png" alt="Kubuno Contacts logo" width="120">

# Kubuno — Contacts

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-edition_2021-orange.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)
![Status](https://img.shields.io/badge/status-alpha-yellow.svg)
![Module](https://img.shields.io/badge/Kubuno-module-4D38DB.svg)

**A full-featured address book for [Kubuno](https://github.com/kubuno/core) — the self-hosted, libre (AGPLv3) cloud platform, a sovereign alternative to Google Workspace and Microsoft 365.**

Rich contact records, groups and labels, an instance-wide directory, CardDAV synchronization with your phone and desktop clients, and a shared contact picker that every other module can reuse — all on your own server.

</div>

---

## ✨ Features

- 👤 **Full-featured address book** — rich contact records (names, pronouns, organization and job title, emails, phones, postal addresses, URLs, important dates, related people, instant-messaging handles, custom fields, notes and avatars), with starring, archiving, blocking, and a trash you can restore from.
- 📇 **A contact on its own page** — each contact opens at its own address (`/contacts/person/<id>`) with the whole area to itself and a real URL to share or bookmark; the editor is a single Material-style form with progressive disclosure, two-column layout on wide screens, and a country dial-code selector for phone numbers.
- 🗂️ **Groups & labels** — organize contacts into colored groups and freeform labels, both managed straight from the sidebar. Every view (all, starred, a group, a label, a smart view…) has its own shareable URL, so deep links and the browser Back button just work.
- 🧠 **Smart views** — birthdays, frequently contacted, follow-up suggestions, and a duplicate finder with one-click merge or ignore.
- 🔔 **Reminders** — birthday and custom follow-up reminders with recurrence, surfaced as a due-count badge in the sidebar.
- 🔄 **Import & export** — vCard (`.vcf`) and CSV import; vCard and CSV export of your whole address book.
- 📱 **CardDAV** — sync your contacts with phones and desktop clients through the built-in CardDAV endpoint, with per-user tokens (toggleable instance-wide).
- 🏢 **Instance directory & organizational unit** — browse the other people on your Kubuno instance, and a dedicated "My organizational unit" group reads the core's governed directory (the instance's sharing policy still decides what is visible). Add anyone to your own address book in one click; no copy of the account list is kept.
- 🤝 **"Other contacts"** — people you have actually dealt with through another module (a mail correspondent, someone met in a chat) but never saved, fed entirely through the **core's event bus** — so mail and chat need to know nothing about the address book. Keep them or dismiss them.
- 🔗 **Sharing** — share a contact with other users of the instance, or through a public link (optionally password-protected and time-limited, subject to admin policy).
- 🧩 **Cross-module integration** — other modules can open a globally-mounted **contact picker** published on the core's service registry, a contact copied with "Copy for Kubuno" pastes as a **rich contact card** in consumer modules such as Chat or Notes, and Contacts feeds a **person card** so a guest shown elsewhere carries their job title, organization, phone, address and photo. Everything degrades gracefully when Contacts is not installed.
- 💬 **@mention provider** — typing `@` in any mention-enabled text field across the platform suggests your contacts with avatar and email and inserts them as a removable chip, published on the core's extension registry.
- ⚡ **Delta sync** — cursor-based `/delta` endpoints (contacts, labels, groups, reminders) with monotonic change sequences and tombstones, powering incremental pulls by local-first clients; client-minted IDs are honored on create for offline replay.

## 🏗️ Architecture

Kubuno is **modular**: a **core** (the platform's "operating system") plus independent **modules**. Each module — Contacts included — is a **separate process** that connects to the core at startup on its own dedicated port (**3110** for Contacts); the core proxies its routes (`/api/v1/contacts/*`), distributes events and serves its runtime-loaded React frontend bundle.

- **Backend** — `src/`: Axum + SQLx (PostgreSQL, schema `contacts`); migrations in `migrations/`.
- **Frontend** — `frontend/`: a React bundle built to `entry.js`, consuming `@kubuno/sdk`, `@ui` and `@kubuno/drive` from the host at runtime via its import map.

## 📦 Install

The easiest way to self-host a full Kubuno instance (core + every module, Contacts included) is the **all-in-one Docker image** (`ghcr.io/kubuno/kubuno`). See **[kubuno/docker](https://github.com/kubuno/docker)** for `docker compose` instructions.

To add this module to an existing instance, install its **Kubuno package** (`.kbpkg`) — the single format the core installs by itself, the same file on Linux, Windows and macOS. Grab it from the admin console's marketplace, or install it offline from the command line:

```bash
sudo kubuno modules:install dist/contacts-<version>-<os>-<arch>.kbpkg
sudo systemctl restart kubuno         # the core loads the module on (re)start
```

The `.kbpkg` is a ZIP archive rooted at the module folder; the core unpacks it in pure Rust, so installation is identical on every platform. It is the **only** distribution format for a module — a module is not a system service, so there are no `.deb`/`.rpm`/`.exe`/`.pkg` packages.

## 🛠️ Build & development

**Requirements:** Rust ≥ 1.82, Node.js ≥ 24, PostgreSQL 16.

```bash
cargo build --release                     # → target/release/kubuno-contacts (shared crates from git tags)
cd frontend && npm ci && npm run build    # → dist/{entry.js, entry.css} (@kubuno/* from npm)
bash build_kbpkg.sh                       # → dist/contacts-<version>-<os>-<arch>.kbpkg
bash build_kbpkg.sh --install             # build, install into the local module store, and restart
```

> Shared dependencies come from Kubuno — no `kubuno/core` checkout required:
> - **Rust** — shared crates via tagged git dependencies on `kubuno/core`.
> - **Frontend** — `@kubuno/sdk`, `@kubuno/ui` and `@kubuno/drive` from the `@kubuno` npm scope, resolved at runtime to the host's singletons through its import map.

## 📦 Tech stack

Rust 2021 · Axum 0.7 · Tokio · SQLx 0.8 (PostgreSQL 16, schema `contacts`) — React 19 · TypeScript · Vite · Tailwind CSS v4 · Zustand · React Query.

## 🤝 Contributing

Contributions are welcome. Please open an issue to discuss any significant change before submitting a pull request.

## 📄 License

[AGPL-3.0-or-later](LICENSE) © Kubuno contributors.
