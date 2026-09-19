# Changelog

All notable changes to **kubuno-contacts** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this
project adheres to [Semantic Versioning](https://semver.org/). Entries are added under
`[Unreleased]` **as the change is made**; `_tools/release.sh` stamps them under the version
number at release time, and CI publishes that section as the GitHub Release notes.

## [Unreleased]

### Security

- **Input validation library updated.** The version in use carried
  RUSTSEC-2024-0421 through its domain-name parser, which accepted Punycode
  labels that decode to plain ASCII — a mismatch an attacker can use to make two
  different names look like one.

## [0.1.8] - 2026-09-18

### Security

- **HTTP/2 layer updated to a patched release.** `h2` moves from 0.4.15 to
  0.4.19, closing a denial of service through unbounded empty DATA frames
  (RUSTSEC-2026-0258).
- **Error library updated to a patched release.** `anyhow` moves from 1.0.102
  to 1.0.104, closing an unsoundness in `Error::downcast_mut()`
  (RUSTSEC-2026-0190).
- **TLS library updated to a patched release.** The pinned `rustls` carried
  RUSTSEC-2026-0285 (medium). Every outbound HTTPS connection goes through it.

## [0.1.7] - 2026-09-18

### Added

- **Contacts tells other modules what it knows about a person.** When something
  elsewhere shows someone — a guest on a calendar event today — their job title,
  organisation, department, telephone, address and photo come from your contacts,
  along with a link to their site and a way to open their full sheet. Nothing here
  names the module that asks.

### Changed


- **This module now installs as a Kubuno package (`.kbpkg`) only.** Its system
  packages (Debian/RPM and the Windows and macOS installers) are no longer
  built: the module is distributed as one `.kbpkg` per platform (Linux, Windows,
  macOS) that the Kubuno server installs itself — from the admin console, or
  offline with `kubuno modules:install <file>.kbpkg`.
- **The README now opens with the module's logo.** The public README on
  GitHub now shows the module's designer logo (the same PNG shown as the
  browser tab icon and in the applications menu) at the top of the page — the
  repository landing now matches the icon a signed-in user sees inside the
  platform. The image ships in-repo, under `.github/logo.png`, so it renders
  even when the repo is browsed offline.

- **New Contacts logo** — a blue hexagon with a white person silhouette,
  used as the browser-tab icon and in the applications menu. It is now raster
  (PNG) designer artwork.




### Fixed


- **A withdrawn dependency is no longer used.** A crate deep in the tree
  (`spin` 0.9.8, pulled in through the HTTP stack) was yanked by its authors.
  No vulnerability was announced, but a withdrawn crate has no business in a
  release; the lockfile now takes the version that replaced it.
- **The package could not be built where `zip` is absent.** The Windows job of
  the continuous integration has no `zip`, so the Windows package was simply lost
  the first time it was attempted — a script failure, not a build failure. The
  builder now falls back to 7-Zip, then to PowerShell.
### Added

- **This module now ships a `.kbpkg`** — the single package format a Kubuno
  server installs by itself, the same file on Linux, Windows and macOS. It
  carries the same binary, interface and manifest as the system packages,
  arranged the way the server expects to find a module on disk, plus a
  `SHA256SUMS` so a copy carried offline can be checked without the catalogue.
  Nothing changes for existing installations: the `.deb`, `.rpm`, `.exe` and
  `.pkg` are still published, and a catalogue that sees both simply prefers the
  new one. It is also the only format the server can unpack without an external
  tool, which is what makes one-click installation possible away from
  Debian-like systems.
### Fixed

- **A built package could be thrown away instead of published.** The job that
  attaches a package to the release waited ten minutes for another workflow to
  create that release, then gave up with "release never appeared — build.yml
  likely failed". The diagnosis was wrong: on a repository whose `.deb` takes
  longer than ten minutes to build, the release simply did not exist yet, and a
  package that had built perfectly was discarded. Four modules reached v0.1.6
  with packages missing for some systems because of it. The job now creates the
  release itself when it is missing, so it no longer depends on another workflow
  finishing first.
### Added

- **Security policy and CI quality gate.** A `SECURITY.md` documents how to
  report vulnerabilities, and a CI workflow enforces `clippy -D warnings`, a
  dependency-vulnerability audit (`cargo audit`) and the frontend typecheck/tests.

- **"Other contacts"**, a new sidebar group: people you have actually dealt with
  through another module — a mail correspondent, someone met in a chat — who were
  never saved into the address book. Each one can be kept (it becomes a real
  contact) or dismissed, and anyone already saved never shows up. The list is fed
  through the **core's event bus**: a module publishes that the user exchanged
  with someone, so mail and chat need to know nothing about the address book, and
  an instance without one simply has no subscriber.
- **"My organisational unit"**, a second new group: everyone in the signed-in
  user's own unit and its sub-units, read from the core's governed directory —
  the instance's sharing policy still decides what is visible, and this module
  keeps no copy of the account list. Members can be pulled into the address book
  one by one.

- **Rebuilt mobile experience.** A bottom navigation bar (Contacts, Frequent,
  Directory) in portrait — a left rail in landscape — puts the primary views one
  thumb-tap away; the secondary views stay in the drawer. Opening a contact is
  now a full screen with its own back arrow rather than a panel with a small
  close button, and a "+" in the header creates one (the desktop's sidebar button
  is in the drawer on mobile, and the shell already owns the bottom-right corner).
  List rows are taller, touch-sized, and show the email under the name where the
  desktop's columns do not fit; the table density, a desktop affair, is hidden on
  a phone.

### Changed

- **Pill-shaped buttons are gone from the interface.** Filter chips, view
  segments, tab selectors and action buttons that were drawn as pills now use the
  same 4 px corner radius as every other button — the shape set them apart for no
  reason other than habit. Round buttons that hold a lone icon, avatars, status
  dots and non-clickable badges keep their shape: a circle around a single glyph
  is not a pill.

- **A contact opens on its own page** (`/contacts/person/<id>`) instead of a
  380px strip pinned beside the list. The card gets the whole area, it has a real
  address to share or bookmark, and the browser's Back button returns to the view
  it was opened from. A direct link loads the contact on its own, even when it is
  not part of the current list.

- **The contact editor was rebuilt as a single form**, in the shape people expect
  from an address book. One icon column, Material fields with floating labels,
  and progressive disclosure everywhere: **Name** and **Organisation** are groups
  whose chevron reveals the rarely-used parts (prefix, middle name, suffix,
  nickname, pronouns / department), and a **"More"** switch at the foot adds
  important dates, websites, related people and custom fields. Each repeated
  value carries a **cross on hover** to drop it, an empty section offers a full
  width button while a filled one offers a discreet link, phone numbers get a
  **country dial-code selector** and the birthday is entered as day / month /
  year with the year optional. Labels can be attached straight from the form.
  The form is a view of the module, laid out in the content area with the
  navigation still in place — not a window over the application. A **star**
  marks the contact as a favourite before it even exists, the **photo** is
  shown at full size with a "+" badge that opens the image picker, and
  **labels can be created straight from the form** rather than only in the
  sidebar. On a wide screen the form **splits into two columns** — who the
  person is on the left, how to reach them on the right — instead of leaving
  half the page empty; a narrower window folds back to a single column in the
  same reading order. The avatar's colour now opens the **platform's colour
  picker** — wheel, harmonies, hex and remembered custom colours — instead of
  ten frozen swatches.

- The selected view now comes from one place, derived from the URL, so a sidebar
  link, a bottom-nav tab, a deep link and the Back button all agree. The
  `/contacts/starred`, `/contacts/trashed` and new `/contacts/directory` routes
  are real destinations again (they used to render but not select their view).

### Fixed

- On a block of stacked fields — a postal address — the section icon and its
  remove cross drifted to the middle of the stack; they sit on the first field
  again.

### Security

- The event endpoints the core delivers to (`/ipc/events` and its `/events`
  fallback) now both require the internal secret. `/events` was reachable
  without it, so anyone able to call the module could inject forged events —
  fabricated correspondents would have appeared in "Other contacts".

## [0.1.6] - 2026-08-19

### Changed

- Theme tokens: two colours for navigation labels (`--color-text-nav`,
  `--color-text-nav-active`). Every module carries the same token sheet, so the
  values must match across them — whichever bundle loads last would otherwise
  win. No visible change inside this module.

### Added

- A **mini-panel for the shell's right rail**: a search box and the starred contacts,
  with the two actions worth having in place — copy the address, start a call.

### Changed

- Default application background token aligned with the core (`--body-bg` `#f8fafd`). Only
  visible when the module runs standalone: inside the shell the active theme sets it.

[Unreleased]: https://github.com/kubuno/contacts/compare/v0.1.8...HEAD
[0.1.8]: https://github.com/kubuno/contacts/releases/tag/v0.1.8
[0.1.7]: https://github.com/kubuno/contacts/releases/tag/v0.1.7
[0.1.6]: https://github.com/kubuno/contacts/releases/tag/v0.1.6
