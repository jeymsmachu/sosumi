# 0001 — One process, not a separate engine

- Status: Accepted
- Date: 2026-11-13

## Context

The reference client, Adam, runs its audio engine as a separate Go process that
outlives the window. That choice serves an Electron shell well: the engine owns
playback and can keep going when the renderer reloads. The cost is a lifecycle
to manage — a lock file, orphan detection by reading `/proc`, and a loopback
HTTP surface that needs origin checking and peer-credential validation.

Sosumi is a native application: a Flutter UI over a Rust core, compiled into one
binary. There is no renderer reload to survive.

## Decision

Sosumi is one OS process. The UI and the core share it.

Closing the window ends playback, MPRIS, and Discord presence. There is no
background daemon and no sidecar.

## Consequences

**Easier:** no lock file, no orphan recovery, no HTTP surface, no peer-credential
checks. One lifecycle to reason about. Lower idle memory. The app can never leak
a process.

**Harder:** playback cannot continue once the window is closed. Users who expect
music to keep playing after closing the window will be surprised. A future
mini-player or tray mode keeps the process alive; that is a UI decision, not an
architectural one.

**Accepted:** the loss of background playback as a default. It is documented
rather than worked around.
