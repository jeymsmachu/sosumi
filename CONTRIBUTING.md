# Contributing to Sosumi

Thanks for helping. A few things make the project easier to work on.

## Before you start

Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
[docs/LEGAL-SCOPE.md](docs/LEGAL-SCOPE.md). The second one matters: it explains
what this project will and will not accept.

## Ground rules

1. **No bundled DRM components, device credentials, or Apple binaries.** Ever.
2. **Optional features stay optional.** If a change makes the app depend on
   `FairPlay`, Cavern, or Discord to build or start, it will be declined.
3. **Keys never leave `sosumi-security`.** Do not add a function that returns a
   key to another crate.
4. **One FFI surface.** The UI talks to `sosumi-bridge` only.

## Code style

- Rust: `cargo fmt` and `cargo clippy --workspace --all-targets`. The workspace
  denies `unsafe`, `unwrap`, `expect`, and `panic` in non-test code.
- Dart: `dart format` and `flutter analyze`.
- Comments explain *why*. The code should say what.

## Tests

Every crate keeps its unit tests beside the code. A change with new logic should
come with tests, and a bug fix should come with a test that fails before it.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Commits

Conventional, scoped, and readable:

```
feat(player): gapless transition between ALAC tracks
fix(security): reject truncated sealed files instead of panicking
docs(adr): record the audio-stack decision
```

## Adding a provider

Implement `MediaProvider` from `sosumi-provider-api`. Nothing above that crate
needs to change: the player, library, and UI hold `dyn MediaProvider`.

## Adding a capability

Add a variant to `Capability`, a probe, and a row appears in Settings. If it
requires an install step, keep the install explicit and out of the default path.

## Reporting bugs

Include your distribution, the output of `cargo --version` and `flutter
--version`, and the log from `SOSUMI_LOG=debug`. Reproduce on a build from
`main` where you can.
