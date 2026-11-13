# Architecture

## Shape

Sosumi is one OS process. The Flutter UI and the Rust core live in the same
binary, connected by a single generated FFI surface.

```
┌──────────────────────────────────────────────────────────┐
│  apps/desktop — Flutter                                   │
│    features/  player · library · search · settings         │
│    theme/     Liquid Glass materials                       │
└──────────────────────┬───────────────────────────────────┘
                       │  sosumi-bridge  (flutter_rust_bridge v2)
┌──────────────────────▼───────────────────────────────────┐
│  sosumi-core                                              │
│    orchestration · queue · event bus · session            │
├────────────┬────────────┬──────────────┬─────────────────┤
│ sosumi-    │ sosumi-    │ sosumi-      │ sosumi-         │
│ audio      │ library    │ spatial      │ security        │
├────────────┴────────────┴──────────────┴─────────────────┤
│  sosumi-provider-apple · sosumi-provider-local            │
│            (both implement sosumi-provider-api)           │
├──────────────────────────────────────────────────────────┤
│  sosumi-storage · sosumi-metadata · sosumi-diagnostics    │
├──────────────────────────────────────────────────────────┤
│  sosumi-linux (MPRIS2) · sosumi-discord (Rich Presence)   │
└──────────────────────────────────────────────────────────┘
```

## Dependency rules

These are enforced by inspection today and by a test once the graph is stable.

1. **`sosumi-diagnostics` depends on nothing.** It is the root; every other crate
   may use it, so it must never reach upward.
2. **`sosumi-provider-api` owns the vocabulary.** Domain types and the
   `MediaProvider` trait live here. Nothing above it knows what Apple is.
3. **`sosumi-security` is a leaf.** Key material never leaves it. Callers get a
   decrypting reader, never a key.
4. **The UI depends on `sosumi-bridge` and nothing else.** One FFI surface keeps
   codegen small and the boundary auditable.
5. **No crate reads the network except a provider.** Anything needing HTTP lives
   behind `sosumi-provider-apple`.

## The two-axis model

Audio capability and process architecture are independent. Sosumi is designed
around this table:

| Tier | What it serves | Component it needs |
|---|---|---|
| Baseline | AAC | A Widevine device identity |
| Lossless / Hi-Res | ALAC | Apple's Android `FairPlay` libraries |
| Spatial Audio | E-AC-3 JOC, rendered binaurally | An external object-based renderer |

Every tier above the baseline is represented as a
[`Capability`](../crates/sosumi-security/src/capability.rs) with a
runtime-detected state. The app builds and runs with none of them installed, and
the Settings screen renders the registry directly. This is what makes "optional
DRM" an architectural property rather than a promise.

## Data flow for one track

```
UI: play(track)
  └─ sosumi-core::Player::play
       ├─ provider.open(StreamRequest { quality })
       │    └─ Apple: catalog → HLS master → variant pick
       │         ├─ AAC      → licence exchange → content key
       │         └─ ALAC     → FairPlay shim (if installed)
       └─ sosumi-audio::Player::load(session)
            ├─ decode   (Symphonia)
            ├─ render   (sosumi-spatial, if the stream is object-based)
            └─ output   (cpal → ALSA/PipeWire)
```

The audio layer only ever sees decrypted frames and a `StreamInfo`. It has no
concept of DRM, quality tiers, or Apple.

## Why one process

Adam runs a separate Go engine so it can own playback independently of the
window. That design is well-suited to an Electron shell. Sosumi is a native
process with a native window, so the same separation buys nothing and costs a
lifecycle to manage: lock files, orphan recovery, and an HTTP surface that must
be authenticated. One process, one lifecycle.

The cost is that closing the window stops playback. That is the intended
behaviour, and it is documented rather than worked around.

## Decisions

See `docs/adr/` for the reasoning behind the larger choices.
