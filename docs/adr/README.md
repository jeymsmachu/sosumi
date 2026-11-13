# Architecture Decision Records

Short records of decisions that are expensive to reverse. Newest investigations
first, but read in order for the reasoning.

| # | Decision | Status |
|---|---|---|
| [0001](0001-single-process.md) | One process, not a separate engine | Accepted |
| [0002](0002-rust-core-flutter-ui.md) | Rust core with a Flutter UI over one FFI surface | Accepted |
| [0003](0003-optional-capabilities.md) | DRM and Spatial Audio are optional, runtime-detected capabilities | Accepted |
| [0004](0004-widevine-device-identity.md) | The user supplies the Widevine device identity | Accepted |
| [0005](0005-offline-sealing.md) | Downloads are re-sealed under a keyring-held key | Accepted |
| [0006](0006-native-audio-stack.md) | Symphonia + cpal, not libmpv or GStreamer | Accepted |

## Template

```markdown
# NNNN — Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context

What forces are at play.

## Decision

What we chose.

## Consequences

What becomes easier, what becomes harder, and what we accept.
```
