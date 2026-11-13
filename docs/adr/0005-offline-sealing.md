# 0005 — Downloads are re-sealed under a keyring-held key

- Status: Accepted
- Date: 2026-11-13

## Context

`FairPlay` issues no portable licence, so an offline copy cannot remain in its
original protection. A downloaded track must be decrypted once and re-encrypted
under a key Sosumi controls.

The question is where that key lives. Options: the OS keyring, a user
passphrase, or a TPM where available.

## Decision

A random 256-bit key, generated on first use, stored in the OS keyring through
the Secret Service, and used to seal each file with AES-256-GCM. The header is
bound as associated data so a truncated or edited file fails authentication
rather than decrypting to garbage.

A passphrase was rejected: it breaks background playback and download resumption
by requiring an unlock per session.

A TPM was rejected for now: it fragments the build across hardware and adds a
platform dependency for a marginal gain against the stated threat model.

## Consequences

**Easier:** downloads work while the user session is unlocked, which is the
normal case. The format is versioned and key-id tagged, so key rotation is a
migration rather than a rewrite.

**Harder:** a headless environment without a Secret Service needs an explicit
key, so the storage layer takes a key rather than looking one up itself.

**Accepted:** this protects against casual copying and other userland processes,
not against a user who controls the machine. `SECURITY.md` states this plainly
and the app never claims otherwise.
