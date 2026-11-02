# Token service

`services/token` is the deployment-owned MusicKit developer-token minter. It
holds Apple's `.p8` key and returns short-lived ES256 JWTs. It never receives or
stores a user's Music User Token.

## Endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/health` | Liveness/version |
| `POST` | `/v1/developer-token` | Issue a developer token |

Request body is optional origin binding:

```json
{
  "origin": ["https://music.example.com"]
}
```

The `origin` claim is what Apple recommends for web clients. Omit it for native
clients.

Response:

```json
{
  "token": "eyJhbGciOiJFUzI1NiIsImtpZCI6...",
  "expires_at": 1760000000,
  "expires_in": 3600
}
```

## Configuration

| Variable | Required | Default | Notes |
|---|---:|---|---|
| `APPLE_TEAM_ID` | yes | — | 10-character Apple Developer Team ID |
| `APPLE_KEY_ID` | yes | — | 10-character MusicKit key ID |
| `APPLE_PRIVATE_KEY_PATH` | one of | — | Path to the `.p8` key |
| `APPLE_PRIVATE_KEY_PEM` | one of | — | Inline PEM; preferred for secret managers |
| `SOSUMI_TOKEN_BIND` | no | `127.0.0.1:8787` | Listen address |
| `SOSUMI_TOKEN_TTL_SECONDS` | no | `3600` | 1 second to 6 months |
| `SOSUMI_TOKEN_API_KEY` | no | — | Require `x-sosumi-key` on requests |
| `SOSUMI_TOKEN_ORIGINS` | no | — | Comma-separated CORS origins, or `*` |
| `RUST_LOG` | no | `sosumi_token=info` | Tracing filter |

For browser clients, set `SOSUMI_TOKEN_ORIGINS` to the exact app origins. Do not
use `*` in production.

## Run

```sh
export APPLE_TEAM_ID=TEAMID1234
export APPLE_KEY_ID=KEYID12345
export APPLE_PRIVATE_KEY_PATH=/secure/AuthKey_KEYID12345.p8
export SOSUMI_TOKEN_ORIGINS=http://localhost:8080

cargo run -p sosumi-token
```

```sh
curl -s http://127.0.0.1:8787/health
curl -s -X POST http://127.0.0.1:8787/v1/developer-token \
  -H 'content-type: application/json' \
  -d '{}'
```

## Security

- The `.p8` key is never shipped in Flutter.
- Put the service behind TLS and a rate limiter. The optional API key is for
  server-to-server deployments; a browser-side key is not a secret.
- Logs never include the token or key.
- Responses use `Cache-Control: no-store`.
- Rotate the MusicKit key in the Apple Developer portal if it is exposed.
