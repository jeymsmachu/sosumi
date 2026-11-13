# 0006 — Native audio stack: Symphonia + cpal

- Status: Accepted
- Date: 2026-11-13

## Context

The core must decode AAC and ALAC and output bit-perfect audio on Linux. Four
options were considered:

1. **Symphonia + cpal** — pure-Rust decode, direct ALSA/PipeWire output.
2. **libmpv** (`media_kit`) — battle-tested, handles HLS and every codec, but a
   large external dependency.
3. **GStreamer** — very capable, good PipeWire integration, large runtime.
4. **libvlc** — what the reference client uses for lossless output, but it brings
   a VLC subset and a heavier process model.

## Decision

Option 1. Symphonia decodes (it supports AAC and ALAC natively), and cpal is the
output path to ALSA and PipeWire.

## Consequences

**Easier:** no C audio dependency, no bundled media framework, no version skew
with the host. Decoding is in-process, which suits the single-process decision
(0001) and keeps resource use low. The decoder is a library call, so the audio
graph is testable without a device.

**Harder:** we own the pieces a media framework would provide — gapless handling,
sample-format conversion, buffering, and the resampler. Object-based audio is the
sharpest edge: Symphonia does not decode E-AC-3 JOC, so Spatial Audio is handed
to an external renderer rather than decoded here.

**Accepted:** more code to own in exchange for a small, predictable, pure-Rust
core.
