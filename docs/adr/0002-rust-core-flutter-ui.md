# 0002 — Rust core with a Flutter UI over one FFI surface

- Status: Accepted
- Date: 2026-11-13

## Context

Two languages are in play: Rust for the audio, provider, and DRM logic, and Dart
for the interface. Something must bridge them.

Options considered:

1. Manual `dart:ffi` with a hand-written C ABI.
2. `flutter_rust_bridge` v2 with code generation.
3. A local HTTP service between a Dart UI and a Rust core.

## Decision

`flutter_rust_bridge` v2, with **one** bridge crate, `packages/sosumi-bridge`,
that is the only thing the UI links. The UI never names another Rust crate.

## Consequences

**Easier:** async support and rich types come for free; the FFI surface is
generated and consistent. Because there is exactly one bridge crate, the surface
stays small and can be versioned like an API. Moving logic across the boundary is
a normal refactor rather than an FFI project.

**Harder:** a codegen step in the build, and an extra crate to keep compiling.
Types that cross the boundary must stay bridge-friendly, which occasionally means
a small adapter type rather than exposing a core type directly.

**Rejected:** a local HTTP service would mirror Adam's engine and reintroduce the
problems 0001 exists to avoid — a port, threat surface, and a second lifecycle.

**Rejected:** hand-written FFI scales badly and makes every model change a
marshalling chore.
