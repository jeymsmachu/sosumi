# Sosumi

> The sound a Mac makes when it can't do something. This one does it anyway.

An unofficial Apple Music client for Linux. Native, low-resource, and glass.

<div align="center">

**Not affiliated with, authorized by, endorsed by, or sponsored by Apple Inc.**

Requires an active Apple Music subscription. Sosumi is not a downloader for
piracy; it is a client for a service you already pay for.

</div>

---

## Status

**Early development.** Milestone 1 (search → play AAC → system controls → glass
shell) is in progress. Nothing here is ready to use yet. Watch releases rather
than building from `main` if you want something that runs.

## What this will be

- **Native, single process.** Flutter UI and the Rust core share one OS process.
  No hidden daemon, no sidecar that outlives the window.
- **Low resource use.** Pure-Rust audio decode and direct ALSA/PipeWire output.
- **Liquid Glass UI.** Apple-style materials, built to Apple's own guidelines,
  without pretending to be an Apple product.
- **Lossless and Hi-Res, opt-in.** Lossless playback needs components you
  install yourself; the app works without them and tells you what's missing.
- **Spatial Audio.** Object-based audio rendered binaurally. Off by default and
  never bundled, because the renderer has its own licence.
- **Offline downloads.** Encrypted at rest with a key held in your system
  keyring.
- **Honest about DRM.** See [docs/LEGAL-SCOPE.md](docs/LEGAL-SCOPE.md). This
  repository contains no circumvention code, no extracted CDMs, and no Apple
  binaries.

## Documentation

| Document | Contents |
|---|---|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | How the pieces fit and why |
| [PROVIDERS.md](docs/PROVIDERS.md) | The provider contract and how to add one |
| [SECURITY.md](docs/SECURITY.md) | Threat model and key handling |
| [PRIVACY.md](docs/PRIVACY.md) | What leaves the machine, and what does not |
| [PERFORMANCE.md](docs/PERFORMANCE.md) | The budget and how to measure it |
| [LEGAL-SCOPE.md](docs/LEGAL-SCOPE.md) | What this project does and does not do |
| [PLATFORM-LIMITS.md](docs/PLATFORM-LIMITS.md) | What cannot work on some systems, measured, and why |
| [adr/](docs/adr/README.md) | Decisions that are expensive to reverse |

## Capabilities

Sosumi is built so the core application never hard-depends on anything optional.
The Settings screen detects each capability and offers a one-click install where
one applies.

| Capability | Default | Requires |
|---|---|---|
| AAC streaming | On | Apple Music subscription |
| Lossless & Hi-Res | Opt-in | Optional components, second sign-in |
| Spatial Audio | Opt-in | An external object-based renderer |
| System media controls (MPRIS2) | On | D-Bus |
| Discord Rich Presence | Opt-in | Discord running |

## Requirements

- Linux x86_64, glibc, ALSA or PipeWire
- An active Apple Music subscription

## Building

```bash
cargo build --workspace          # Rust core
cargo test --workspace           # Tests
flutter build linux              # UI (once the shell lands)
```

## Repository layout

```
crates/       Rust core. One concern per crate; see docs/ARCHITECTURE.md
packages/     sosumi-bridge — the only FFI surface the UI links
apps/desktop/ Flutter application
docs/         Architecture, security, legal scope, and ADRs
packaging/    Flatpak, AppImage, deb, rpm
```

Each crate has a single responsibility and a narrow dependency budget. The UI
depends on exactly one crate, `sosumi-bridge`, so the FFI surface stays small
and versioned like an API.

## Credits

Sosumi is an independent implementation. Its design was informed by studying
these projects; no code was copied except where a licence permits and is noted.

| Project | Licence | What we learned |
|---|---|---|
| [Adam](https://github.com/silentone12725/apple-music-linux) | MIT | End-to-end Apple Music on Linux; DRM layout; Cavern integration |
| [musickit-sdk-linux](https://github.com/silentone12725/musickit-sdk-linux) | MIT | Engine decomposition and the HTTP API shape |
| [WorldObservationLog/wrapper](https://github.com/WorldObservationLog/wrapper) | MIT | The `FairPlay` exchange the optional tier builds on |
| [libhybris](https://github.com/libhybris/libhybris) | Apache-2.0 | Loading bionic libraries from a glibc process (embedded) |
| [Cavern](https://github.com/VoidXH/Cavern) | Custom (non-commercial, attribution) | Object-based rendering and the pipe protocol |
| [Symphonia](https://github.com/pdeljanov/Symphonia) | MPL-2.0 | Pure-Rust audio decode |
| [AMLL](https://github.com/amll-dev/applemusic-like-lyrics) | AGPL-3.0 | Lyrics presentation ideas only; no code used |
| [flutter_lyric](https://pub.dev/packages/flutter_lyric) | MIT | Word-timed lyric rendering |
| [liquid_glass_widgets](https://pub.dev/packages/liquid_glass_widgets) | MIT | Liquid Glass shaders |

Apple, Apple Music, and `FairPlay` are trademarks of Apple Inc. Dolby and Dolby
Atmos are trademarks of Dolby Laboratories. Sosumi uses neither name in its UI;
object-based audio is presented as "Spatial Audio".

## Licence

MIT. See [LICENSE](LICENSE). Third-party components and their terms are
listed in [NOTICE.md](NOTICE.md).
