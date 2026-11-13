# Providers

A provider is a source of music. Two exist today, and adding a third means
implementing one trait.

## The contract

`MediaProvider` in `crates/sosumi-provider-api/src/provider.rs` is the whole
surface. It has seven methods, and every one is defined for every provider:

| Method | Purpose |
|---|---|
| `id` | Stable id, matching the prefix in its track ids |
| `name` | Display name |
| `is_available` | Cheap, non-blocking readiness check |
| `search` | Catalog search |
| `track` | One track's metadata |
| `album`, `playlist`, `artist` | Collection detail |
| `lyrics` | Synced lyrics |
| `open` | Acquire a playable stream |

A provider that cannot serve a request returns
`ProviderError::CapabilityMissing` rather than omitting the method. That keeps
the UI free of per-provider special cases: it always calls the same thing and
always gets either data or an actionable error.

## What a provider does not do

- **It does not decode.** `open` returns a description of where the bytes are.
- **It does not touch a device.** Output belongs to `sosumi-audio`.
- **It does not hold keys.** An encrypted stream arrives as a decrypting reader,
  so key material stays inside `sosumi-security`.
- **It does not know about other providers.** Track ids are namespaced
  (`apple:1440833098`, `local:3`), so nothing needs to disambiguate them.

## The providers

### `sosumi-provider-apple`

Apple Music. It speaks the catalog API and the web-playback API, parses HLS
playlists, discovers the developer token from Apple's web player, and chooses the
best asset for the requested quality.

Quality selection degrades honestly: a lossless request for a track with no
lossless master returns the AAC variant rather than failing, and the stream info
reports what was actually chosen.

### `sosumi-provider-local`

The user's own files. It indexes configured directories, recognises audio by
extension, treats a subdirectory as an album, and serves each file directly.

Local files never need a capability: the audio is already on disk, so a lossless
request is satisfied whenever the file itself is lossless.

## Adding a provider

1. Create `crates/sosumi-provider-<name>` depending on `sosumi-provider-api`.
2. Implement `MediaProvider`, returning `CapabilityMissing` for anything you do
   not support yet rather than leaving it unimplemented.
3. Register it in `Sosumi::register`.
4. Test the pieces that parse — playlists, wire formats, id mapping — as pure
   functions, the way the Apple provider tests its HLS parser.
