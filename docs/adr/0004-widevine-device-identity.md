# 0004 — The user supplies the Widevine device identity

- Status: Accepted
- Date: 2026-11-13

## Context

Apple's web playback serves audio protected by Widevine. A licence server
accepts only a device identity it recognises. There are three ways to satisfy
this:

1. **Embed a known identity.** The reference client ships a public emulator
   identity, so playback works with no setup. This is a credential the project
   did not author, and shipping it is a grey area.
2. **Generate one at runtime.** Sosumi could mint an RSA key and a
   self-signed certificate blob. In practice the licence server rejects these:
   they carry no recognised certificate chain.
3. **Require the user to provide one.**

## Decision

Option 3. Sosumi loads the identity from the user's config directory
(`device_private_key` and `device_client_id_blob`) and never ships, generates, or
downloads one.

With no identity present, the app reports the capability as unavailable and
offers setup. It does not fall back to a bundled credential.

## Consequences

**Easier:** the repository contains no device credential, which is consistent
with `LEGAL-SCOPE.md`. The loading path is trivial to test and audit, and a user
with their own identity gets playback immediately.

**Harder:** a fresh install cannot play until an identity is provided. This is
the real cost of the choice, and it is why the Settings flow must be clear.

**Accepted:** worst-case first-run friction, in exchange for the project being
able to say, truthfully, that it ships no credentials and no circumvention code.
