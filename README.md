# sosumi

An unofficial, cross-platform Apple Music client.

Flutter UI. Rust core. Flutter-rendered Liquid Glass on every platform.
Requires an active Apple Music subscription. Not affiliated with Apple Inc.

> Status: scaffold. The app shell, Rust domain core, CI, and manual release
> workflow exist. Playback and Apple Music authentication are not wired yet.

## What is in the repo

```text
sosumi/
├── app/                    # Flutter application (all platforms)
├── core/                   # Rust domain core
├── docs/                   # architecture notes
├── .github/workflows/      # CI, manual release, cache pruning
├── Cargo.toml              # Rust workspace
└── rust-toolchain.toml
```

There are two source trees by design: Flutter for all UI, Rust for the
platform-neutral core. Native playback adapters will live behind the Flutter
platform boundary; they do not add a third application codebase.

## UI

- One Flutter codebase for iOS, iPadOS, macOS, Windows, Linux, Android phone,
  tablet, and Android TV.
- [liquid_glass_widgets](https://pub.dev/packages/liquid_glass_widgets) with the
  iOS 27 glass preset, adaptive quality, and the lightweight shader path on
  Windows/Linux.
- No native SwiftUI surface: keeping Liquid Glass in Flutter avoids a second UI
  language. The result is a close Flutter rendering of Apple’s material, not a
  native system material.
- Glass, ambient motion, density, and accent color are adjustable in Settings.
- The UI is Apple-inspired but not Apple-branded: no Apple logos, trademarks, or
  copied assets.

## Core

The Rust crate owns catalog-independent state and the Apple Music backend:

- typed catalog/library models
- queue and repeat behavior
- playback capabilities and audio-variant labels
- Apple Music REST client: search, albums, artists, playlists, library, recent
  plays, and timed lyrics
- ES256 developer-token signing and session packaging
- a serializable snapshot for the future Dart bridge

It deliberately has no dependency on Apple frameworks, Flutter, or DRM.
Platform playback and DRM stay in the platform adapters. See
[docs/backend.md](docs/backend.md).

The Flutter bridge lives in `app/rust` (FRB adapter) with Cargokit build glue in
`app/rust_builder`. Generated Dart is in `app/lib/src/rust`; `RustLib.init()`
runs before the app starts. See [docs/architecture.md](docs/architecture.md).

The deployment-owned token minter lives in `services/token` and is documented in
[docs/token-service.md](docs/token-service.md).

## Sign-in and playback

- Apple Music account: required for catalog, library, and playback.
- DRM account: a separate sign-in only on platforms that need the compatibility
  path. This is intentionally separate from the Apple Music account so the two
  sessions can be managed independently.
- Playback is gated on an active Apple Music subscription.
- Proprietary components are obtained locally by the user and are never
  committed or redistributed.

| Platform | Playback path |
|---|---|
| iOS / iPadOS / macOS | MusicKit |
| Android phone / tablet | MusicKit for Android, where supported |
| Linux | Compatibility playback path |
| Android TV | Compatibility playback path, ARM |
| Windows | MusicKit on the Web sidecar, with handoff to the official app for lossless/Atmos |

## Development

Prerequisites: Flutter 3.47.5 or newer, Rust stable, and the platform toolchains
you intend to build.

```sh
# Flutter
cd app
flutter pub get
dart format lib test
flutter analyze
flutter test

# Rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## CI

Per-push and pull-request compile checks:

- Rust: fmt, clippy with warnings denied, tests.
- Flutter: format check, analyze, widget tests.
- Debug builds for Linux, Android, macOS, iOS (unsigned), and Windows.

Release builds are manual only:

- **Actions → release-build → Run workflow**.
- Builds Android APK, Linux tarball, Windows zip, macOS app zip, an unsigned
  iOS app zip, and the Linux `sosumi-token` service binary.
- Artifacts are retained for 7 days.
- Android release currently uses the generated debug signing config; replace it
  with a real keystore before distributing.
- iOS output is unsigned.

### Cache budget

GitHub Actions caches are capped at 10 GiB per repository. This repo targets
**8 GiB**:

- Rust caches dependencies only (`cache-targets: false`) and only writes on
  `main`.
- Gradle caches are read-only on pull requests and cleaned before saving.
- Flutter SDK and pub caches are per OS, shared by jobs on the same runner OS.
- `cache-prune` runs weekly and deletes least-recently-used caches above 8 GiB.
- Release artifacts expire after 7 days.

## License

MIT. See [LICENSE](LICENSE).

This license covers only the source in this repository. It grants no rights to
Apple's software, content, services, or trademarks, or to any proprietary
components obtained separately by the user.

## Disclaimer

This project is independent and unaffiliated with Apple Inc. "Apple Music" and
related marks are trademarks of Apple Inc. An active Apple Music subscription is
required. Nothing in this repository grants rights to Apple's content, services,
or proprietary software. Use at your own risk. No warranty.
