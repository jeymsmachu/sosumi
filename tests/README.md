# Tests

Tests that span crates or need real inputs. Per-crate unit tests live beside the
code they cover, which is where most of them are.

| Directory | Contents |
|---|---|
| `integration/` | Cross-crate flows: a provider through the player, the bridge through the core. |
| `fixtures/` | Captured inputs — HLS playlists, API JSON, tiny audio files. |
| `benchmarks/` | `cargo bench` targets for the paths `docs/PERFORMANCE.md` budgets. |

## Fixtures

A fixture must be something a contributor can inspect and reason about:

- **Playlists and API responses** are captured real responses, with anything
  account-specific removed. They live as plain text so a diff is readable.
- **Audio** is synthesised, not ripped. The audio crate's tests generate a WAV in
  code; a fixture is only needed when a real container's quirks matter.
- **No credentials, tokens, or personal data.** Ever.
- **No Apple binaries.** A fixture that needed one would mean a test cannot run
  in CI, which is a reason to test the logic around it instead.
