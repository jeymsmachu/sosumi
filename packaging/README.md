# Packaging

Distribution manifests, one directory per format. Nothing here is populated yet;
this records what each will hold so the shape is agreed before it is built.

| Directory | Format | Notes |
|---|---|---|
| `flatpak/` | Flatpak | The primary target. Needs a manifest, an AppStream metainfo file, and a desktop entry. |
| `appimage/` | AppImage | Portable, no install. |
| `deb/` | Debian/Ubuntu | Built with `cargo-deb` for the Rust side and the Flutter bundle for the UI. |
| `rpm/` | Fedora/RHEL | Built with `cargo-generate-rpm`, mirroring the deb split. |

Every package must satisfy three constraints:

1. **No bundled DRM components.** Apple's libraries and Cavern's renderer are
   user-installed and never shipped. See `docs/LEGAL-SCOPE.md`.
2. **No network at install time.** If a user later chooses to install an
   optional capability, the app does that with explicit consent, not the
   package manager.
3. **The same binary.** A package arranges files; it does not change behaviour.
