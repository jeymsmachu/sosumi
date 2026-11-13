# Security

## Threat model

Sosumi handles three categories of sensitive data, and treats each differently.

| Data | Where it lives | Protection |
|---|---|---|
| Apple session (cookies, tokens) | Secret store | Sealed with AES-256-GCM; key in the OS keyring |
| Widevine device identity | User's config directory | User-supplied; never leaves its directory; never logged |
| Downloaded audio | Data directory | Sealed with AES-256-GCM; key in the OS keyring |
| Content keys | Memory only | Held in the security crate, zeroed on drop, never serialised |

## What the offline seal is and is not

Sealed downloads protect against:

- Casual copying of files off the machine.
- Another userland process that cannot read your keyring.

They do **not** protect against:

- A user who controls the machine and can read process memory or the keyring.
- A modified build of Sosumi.
- Root.

This is stated plainly because claiming otherwise would be dishonest. It is
software protection, not tamper-proof DRM, and it is never described as the
latter.

## Key handling rules

1. Key material never crosses a crate boundary. `sosumi-security` exposes a
   decrypting reader, never a key.
2. Keys are never logged. `Debug` impls redact them, and this is covered by
   tests.
3. Keys are zeroed on drop.
4. No plaintext credential is ever written to disk. The secret store has a test
   that asserts the plaintext does not appear in the file.

## Network

- All HTTP uses TLS with certificate verification. There is no option to
  disable it.
- Requests go only to Apple's endpoints, or to a `CavernPipeServer` Unix socket
  on the local machine.
- There is no listening socket. Sosumi opens no port and answers no request from
  another process.
- No telemetry leaves the machine. See [PRIVACY.md](PRIVACY.md).

## Reporting a vulnerability

Open a private security advisory on the repository rather than a public issue.
Please include reproduction steps and the version you tested.
