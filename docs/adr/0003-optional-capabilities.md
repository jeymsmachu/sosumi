# 0003 — DRM and Spatial Audio are optional capabilities

- Status: Accepted
- Date: 2026-11-13

## Context

Two features pull in components Sosumi cannot ship: lossless playback needs
Apple's Android `FairPlay` libraries, and Spatial Audio needs Cavern, whose
licence forbids sale and requires attribution. The project's stated goal is a
repository with no circumvention code and no bundled Apple binaries.

A design where these features are compiled in by default would contradict that
goal. A design where they are dropped entirely would fail the feature set.

## Decision

Every optional subsystem is a `Capability` with a runtime-detected
`CapabilityState`, held in one registry that the Settings screen renders
directly.

The application must build, run, and play music with **none** of them present.

## Consequences

**Easier:** the repository stays clean; packaging is straightforward; a
distribution can ship Sosumi without touching Apple's or Cavern's terms. Adding
a capability is one enum variant and one probe.

**Harder:** the tier a user gets depends on what they installed, so the UI must
communicate capability state well rather than assume it. Provider code must
fail with an actionable `CapabilityMissing` rather than a generic error.

**Accepted:** a first-run experience where lossless is off until the user opts
in. This is a feature, not a regression: it is the honest presentation of what
the app can legally do.
