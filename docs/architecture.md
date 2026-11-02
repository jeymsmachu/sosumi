# Architecture

## Boundaries

- `app/` is the only UI codebase. It targets iOS, iPadOS, macOS, Windows, Linux,
  Android phone/tablet, and Android TV.
- `core/` is the only Rust codebase. It owns catalog-independent state and
  models.
- Platform playback adapters sit behind Flutter platform channels/FFI. They are
  build glue, not a second application.

## UI and glass

The shell is a responsive Flutter layout:

- compact: bottom dock and mini player
- medium: sidebar and mini player
- expanded: sidebar, content, and a full now-playing panel

Glass rendering uses `liquid_glass_widgets`:

- iOS 27 regular glass preset for the material.
- `AdaptiveGlass` with premium quality on Impeller platforms.
- Lightweight shader path on Windows and Linux.
- Opaque fallback when glass is disabled in Settings.
- Ambient motion is an Aurora background implemented with blurred Flutter
  gradients, not a shader.

No native SwiftUI surface is used. The Flutter renderer is shared across every
platform and approximates the native material rather than pretending to be a
system view.

## Core

`core/src` contains:

- `model.rs`: tracks, audio variants, capabilities, playback states.
- `queue.rs`: deterministic queue with repeat modes.
- `error.rs`: typed core errors.
- `lib.rs`: `Core`, `CoreSnapshot`, version/ABI constants.

The core is intentionally independent of Apple frameworks, Flutter, and DRM.
Playback adapters will translate platform events into this model; the core does
not learn how DRM works.

## Playback adapters (next)

| Platform | Adapter |
|---|---|
| iOS / iPadOS / macOS | Swift MusicKit bridge |
| Android phone / tablet | Kotlin MusicKit for Android bridge |
| Linux / Android TV | Compatibility player process |
| Windows | MusicKit JS sidecar |

The adapter contract is queue, transport, state, now-playing, capabilities, and
quality. The Flutter UI never calls a platform player directly.

## CI

- `ci.yml`: compile checks on push and pull request.
- `release.yml`: manual release artifacts only.
- `cache-prune.yml`: weekly 8 GiB cache budget enforcement.

Cache policy is documented in the repository README. The short version: Rust
caches dependencies only, Gradle is read-only on pull requests, release artifacts
expire in 7 days, and a weekly job deletes least-recently-used caches above the
budget.
