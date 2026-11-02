# Backend

The backend is the Rust crate in `core/`. It owns:

- Apple Music REST access
- ES256 developer-token signing
- account/session packaging
- catalog, library, recently-played, and private timed-lyrics calls

It does not own playback, DRM, or UI state. Those stay in platform adapters and
Flutter.

## Modules

| Module | Responsibility |
|---|---|
| `apple::client` | `AppleMusicClient`: search, song, album, album tracks, artist, artist albums, playlist, playlist tracks, library songs, recently played, storefront, lyrics |
| `apple::models` | typed JSON:API resources and attributes |
| `apple::token` | developer JWT decoding, expiry, and origin claims |
| `apple::signer` | ES256 signing from an Apple `.p8` PKCS#8 key |
| `apple::session` | developer token + Music User Token + storefront |
| `apple::error` | transport/API/auth errors |
| `queue` | deterministic queue and repeat modes |
| `model` | tracks, audio variants, capabilities, playback state |

## Token flow

1. A deployment-owned token service holds the Apple `.p8` key.
2. It creates a short-lived developer JWT with `DeveloperTokenSigner`.
3. The app receives that JWT and uses it for Apple Music REST calls.
4. Apple platforms get the Music User Token from MusicKit; Android/web get it
   from their MusicKit adapters.
5. `AppleMusicSession` packages both tokens and the storefront for the client.

Never put the `.p8` key or a non-rotating developer token in the Flutter app.

## Private lyrics

`AppleMusicClient::lyrics` uses `amp-api.music.apple.com`, the same endpoint the
web player uses. It requires:

- a developer JWT
- a Music User Token
- the `media-user-token` cookie
- the Apple Music origin/referer headers

The method returns TTML. Parsing and animation belong in the Flutter layer; the
Rust layer only fetches and validates the response.

## Example

```rust
use std::time::Duration;
use sosumi_core::apple::{
    AppleMusicClient, AppleMusicSession, DeveloperTokenSigner,
};

let signer = DeveloperTokenSigner::from_pkcs8_pem(
    &std::fs::read_to_string("AuthKey_ABC123.p8")?,
    "TEAMID1234",
    "ABC123DEFG",
    Duration::from_secs(3_600),
)?;
let developer_token = signer.issue_at(unix_now())?;

let session = AppleMusicSession::new(
    developer_token,
    music_user_token,
    "us",
)?;
let client = session.client()?;

let results = client
    .search("daft punk", &[sosumi_core::apple::SearchType::Songs], Some(10))
    .await?;
```

## Testing

`cargo test --workspace` covers:

- queue behavior
- token decoding and expiry
- ES256 signing
- session construction
- Apple Music request headers, parsing, and private-lyrics cookie handling
  against a local `wiremock` server
