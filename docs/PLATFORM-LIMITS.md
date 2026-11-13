# Platform limits

Things the application cannot do on some systems, why, and what to do instead.
Recorded here because each one was found by testing rather than assumed, and
because a limitation a user is not told about looks like a bug in the app.

## Sign-in needs an engine with Encrypted Media

**Symptom.** After entering a correct Apple ID password and the verification code
Apple sends, the page reports:

> Authorization failed. Please log in to enjoy Apple Music.

**Cause.** Apple's sign-in page is not a plain web form. It loads MusicKit on the
Web, which registers a DRM key system as part of starting up and asks the engine
for one of `com.widevine.alpha`, `com.apple.fps`, or `com.microsoft.playready`
through `navigator.requestMediaKeySystemAccess`. When the engine has no Encrypted
Media support, that function does not exist, MusicKit's initialization fails, and
Apple's page reports an authorization error.

The error has nothing to do with the password or the verification code, which is
what makes it so misleading: the user assumes they mistyped something and tries
again.

**Measured, not assumed.** On Arch's `webkit2gtk-4.1` 2.54.1, evaluated inside
the loaded page:

```
api: {"requestMediaKeySystemAccess":false,"mediaKeys":false}
widevine: NO_API
```

`requestMediaKeySystemAccess` is **absent**, under both the default Safari user
agent and a Chrome one. WebKitGTK's build configuration compiles Encrypted Media
only when experimental features are enabled, and the distributed packages build
with the defaults. WPE WebKit is the same.

**What the app does.** It probes for the API as soon as Apple's page loads, before
the user types anything, and states the limitation instead of letting them enter a
password into a flow that cannot finish.

**What to use instead.** A browser engine that ships protected-media support.
Chrome and Chromium do; so do the CastLabs Electron builds that
[Cider](https://cider.sh) and [Sidra](https://github.com/wimpysworld/sidra) use.
On this machine, `~/.config/Sidra/WidevineCdm/` already holds a working
`libwidevinecdm.so`, so the support exists on the system — it is simply not in
WebKitGTK.

## Audio playback needs the subscriber token

**Symptom.** `not authenticated` when playing a track, while search and metadata
work.

**Cause.** Apple's catalogue is readable with an app-level token alone. Playback
is not: it needs the subscriber's token, which only exists after a sign-in.

This is expected and is the correct boundary. It is recorded here so a future
reader does not mistake it for a bug.

## Playback device is absent without a display server

**Symptom.** The app browses and searches, but transport commands report that no
audio output device is available.

**Cause.** The audio engine opens a device through ALSA. A session with no sound
server, such as a bare container, has none.

The app is built so this is not fatal: browsing works, and the reason is reported
as a capability rather than as a crash.

## Recording a new limit

Add a section with three parts: the symptom a person sees, the cause with the
measurement that established it, and what to use instead. A limit without a
measurement is a guess, and guesses age badly.

## Related

- [SECURITY.md](SECURITY.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [`adr/`](adr/README.md)
