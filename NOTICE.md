# Third-party notices

Sosumi is MIT licensed. It incorporates the components below, each of which
keeps its own licence. Attribution here is a licence condition, not a courtesy.

## Apple's FairPlay libraries

**Not included, not redistributed.** `libandroidappmusic.so`, `libCoreFP.so`,
`libCoreLSKD.so`, `libstoreservicescore.so`, and their dependencies are Apple's
property. They are downloaded to the user's own machine on an explicit action,
each file verified against a pinned SHA-256, and used to authenticate an account
the user pays for.

Sosumi's repository contains none of them, and the build does not require them.

## libhybris

| | |
|---|---|
| Project | [libhybris](https://github.com/libhybris/libhybris) |
| Licence | **Apache-2.0** (the components embedded here) plus BSD for two string helpers |
| Where | `crates/sosumi-fairplay/third_party/libhybris-core.so`, embedded as a blob |

libhybris loads libraries built for Android's bionic libc inside a normal glibc
process. It is what makes Apple's x86-64 libraries usable without emulation, a
container, or a second operating system.

The repository ships several licences; only the Apache-2.0 and BSD-licensed
parts are embedded here, and both are compatible with this project's MIT
licence. The GPL-3 and LGPL-2.1 files in that repository are not used.

Copyright: Carsten Munk, Canonical Ltd, Christophe Chapuis, Thomas Perl, Adrian
Negreanu, Franz-Josef Haider, The Android Open Source Project, and Todd C.
Miller. Full text: [`licences/Apache-2.0.txt`](licences/Apache-2.0.txt).

## Design references

Studied for approach, with no code copied except the host-native bridge noted
below. Listed because the debt is real even when the licence is not triggered.

| Project | Licence | What it informed |
|---|---|---|
| [Adam](https://github.com/silentone12725/apple-music-linux) | MIT | The end-to-end shape; the host-native FairPlay bridge |
| [WorldObservationLog/wrapper](https://github.com/WorldObservationLog/wrapper) | MIT | The FairPlay session this project's bridge is derived from |
| [musickit-sdk-linux](https://github.com/silentone12725/musickit-sdk-linux) | MIT | Engine decomposition and API shape |
| [Cavern](https://github.com/VoidXH/Cavern) | Custom, non-commercial, attribution required | Object-based rendering; **not bundled**, see `docs/LEGAL-SCOPE.md` |
| [AMLL](https://github.com/amll-dev/applemusic-like-lyrics) | AGPL-3.0 | Lyrics presentation ideas only; **no code used** |
| [Pake](https://github.com/tw93/Pake) | GPL-3.0 | Its write-up established *why* WebKitGTK cannot host Apple's player |

## Vendored source

`crates/sosumi-fairplay/native/` contains C and C++ derived from Adam's `drm/`
directory and the wrapper's `lite/` directory, both MIT. The header of each file
retains its original copyright. This directory is the only place in the tree
with code that is not ours, it is confined to one crate, and it is the only crate
permitted to use `unsafe`.

Retaining these sources rather than reimplementing them is deliberate: the code
drives Apple's own C++ ABI through many mangled symbols, and a subtle mistake
there breaks sign-in at run time or corrupts memory rather than failing to
compile.

## Why there is no DRM-circumvention code here

Sosumi does not decrypt anything on its own, does not extract keys, and does not
provide a mechanism for playback without a subscription. It drives Apple's own
libraries to authenticate an account and to obtain keys for streams that account
is entitled to. `docs/LEGAL-SCOPE.md` states the scope the project holds itself
to; `docs/SECURITY.md` states what the local protections do and do not cover.
