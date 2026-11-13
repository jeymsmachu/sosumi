# Legal scope

This document states plainly what Sosumi does and does not do, so that
contributors and users can make informed decisions. It is not legal advice.

## What Sosumi is

An unofficial desktop client for Apple Music on Linux. It lets a subscriber use
the service they already pay for on a platform Apple does not support.

## What Sosumi is not

- **Not affiliated with Apple.** No endorsement, no authorization, no
  sponsorship. The README says so; the UI says so.
- **Not a piracy tool.** Sosumi requires an active subscription. It does not
  bypass payment, unlock catalog content for non-subscribers, or provide access
  to anything the user is not entitled to.
- **Not a downloader for redistribution.** Offline files are sealed to the
  user's own machine under a key in their own keyring.
- **Not a source of circumvention code.** This repository contains no DRM
  circumvention implementation, no extracted CDMs, no device credentials, and no
  Apple binaries. See "The optional components" below.

## The baseline tier

Apple's web playback serves audio protected by Widevine. Sosumi speaks the
standard licence-exchange protocol for that protection, using a **device
identity the user supplies**. Sosumi does not ship or generate one:

- No device key or client-ID blob is committed, downloaded, or embedded.
- With no identity configured, the app says so and offers setup. It does not
  fall back to a bundled credential.
- The identity is a property of the user's own client, exactly as it is in any
  browser or player that plays this content.

The intent is to be a **client**, not a key source.

## The optional components

Sign-in, lossless, Hi-Res, and Spatial Audio require components Sosumi does not
ship. Each is user-installed, detected at runtime, and shown in Settings with its
own state. Nothing is fetched silently.

### What the `FairPlay` bridge does and does not do

It drives **Apple's own libraries** to authenticate an account and to obtain keys
for streams that account is entitled to. It is the same mechanism Apple's Android
client uses.

It does **not** decrypt audio independently, extract keys for redistribution,
weaken any protection, or provide any way to play without a subscription. There is
no code in this repository that circumvents a technical protection measure. The
distinction is worth stating because "uses `FairPlay`" and "breaks `FairPlay`" are
different things, and only the first is true here.

Two consequences follow from the second sign-in:

- It authenticates an account the user already pays for. Without an active
  subscription the libraries reject the login; there is no bypass.
- The credentials go to Apple. Sosumi stores the resulting session token, and
  never the password.

| Component | Who provides it | Sosumi's stance |
|---|---|---|
| Sign-in and lossless keys | Apple's Android `FairPlay` libraries | Not redistributed. Fetched only on explicit user action, checksum-verified, and this repository is fully functional without it. |
| Loader for those libraries | [libhybris](https://github.com/libhybris/libhybris), Apache-2.0 | Embedded as a build blob. It is a general-purpose compatibility loader, not a DRM tool. See `NOTICE.md`. |
| Spatial Audio | Cavern (`CavernPipeServer`) | Not bundled. Its licence forbids sale and requires attribution, so Sosumi treats it as an external dependency and credits it. |

## Names and marks

Apple, Apple Music, and `FairPlay` are trademarks of Apple Inc. Dolby and Dolby
Atmos are trademarks of Dolby Laboratories.

- The UI presents object-based audio as **"Spatial Audio"**, never "Dolby
  Atmos", and never uses Apple's or Dolby's marks.
- Lossless and Hi-Res are used descriptively.
- No Apple logos, product names, or trade dress appear in the app, its packages,
  or its metadata.
- Sosumi claims no URL schemes by default. Any scheme association is opt-in, per
  scheme, and explained in Settings.

## Privacy

Sosumi talks to Apple because it must, and to nothing else. There is no
telemetry, no analytics, and no phone-home. Artwork, lyrics, and audio come from
Apple's own endpoints, as they do in any client. See
[docs/PRIVACY.md](PRIVACY.md).

## Contributing

A change that adds a bundled DRM component, an embedded device identity, or
extracted Apple code will be declined. A change that improves the client for
subscribers is welcome.
