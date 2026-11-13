# Privacy

## Short version

Sosumi collects nothing. There is no telemetry, no analytics, no crash
reporting, and no account beyond your Apple ID.

## What leaves your machine

Only what is required to play music:

| Destination | What is sent | Why |
|---|---|---|
| Apple | Requests for catalog, playback, artwork, and lyrics | This is the service you are using |
| Discord | Your current track, if you enable Rich Presence | Optional, off by default, and Discord already knows |
| Local D-Bus | Current track and playback state | So your desktop shows media controls |

There is no other destination. The app has no server component of its own.

## What stays on your machine

- Your Apple session cookies and tokens (sealed).
- Your library cache and downloads (sealed).
- Your settings.
- The Widevine device identity, if you configured one.

All of it lives under the directories `sosumi-storage` resolves, and setting
`SOSUMI_HOME` puts all of it in one place you can inspect or delete.

## What Sosumi does not do

- No analytics SDK, no crash reporter, no unique install id.
- No listening socket. Other processes cannot query the app.
- No background connection when you are not playing or browsing.
- No reading of files outside its own directories.

## Removing your data

Delete the directories under your config, data, and cache paths. Nothing is
stored anywhere else, and nothing survives that deletion.

## Discord Rich Presence

Off by default. When enabled, it reports the current track to the local Discord
client over a Unix socket. It does not contact Discord's servers directly.
