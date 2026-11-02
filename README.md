# sosumi

An unofficial, cross-platform Apple Music client.

Flutter UI. Rust core. Liquid Glass-inspired design on every platform.
Requires an active Apple Music subscription. Not affiliated with Apple Inc.

> Status: pre-alpha. The repository is bootstrapped; the app and core are not
> implemented yet.

## Design

- One Flutter codebase for iOS, iPadOS, macOS, Windows, Linux, Android phone,
  tablet, and Android TV.
- One Rust core for catalog, library, cache, queue, playback state, lyrics, and
  platform-neutral services.
- Native playback adapters where Apple provides them: MusicKit on Apple
  platforms and MusicKit for Android on supported Android devices.
- A compatibility playback path for Linux and Android TV, using the platform's
  existing DRM stack. Proprietary components are obtained locally by the user
  and are never committed or redistributed.
- The UI is Apple-inspired but not Apple-branded: no Apple logos, trademarks,
  or copied assets. Glass, motion, and visual effects are adjustable in
  Settings.

## Sign-in

- Apple Music account: required for catalog, library, and playback.
- DRM account: a separate sign-in only on platforms that need the compatibility
  path. This is intentionally separate from the Apple Music account so the two
  sessions can be managed independently.

Playback is gated on an active Apple Music subscription. The app does not
provide, bundle, or redistribute Apple content, keys, or proprietary binaries.

## Planned layout

```text
sosumi/
├── app/          # Flutter application
├── core/         # Rust core
├── docs/         # architecture and platform notes
└── README.md
```

## Platform notes

| Platform | Playback |
|---|---|
| iOS / iPadOS / macOS | MusicKit |
| Android phone / tablet | MusicKit for Android, where supported |
| Linux | Compatibility playback path |
| Android TV | Compatibility playback path, ARM |
| Windows | MusicKit on the Web sidecar, with handoff to the official app for lossless/Atmos |

## Disclaimer

This project is independent and unaffiliated with Apple Inc. "Apple Music" and
related marks are trademarks of Apple Inc. An active Apple Music subscription is
required. Nothing in this repository grants rights to Apple's content, services,
or proprietary software. Use at your own risk. No warranty.
