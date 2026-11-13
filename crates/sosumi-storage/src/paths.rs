//! XDG-compliant filesystem layout.
//!
//! Every path Sosumi writes lives under a directory derived here, so there is
//! exactly one place to look when auditing what the app touches on disk.

use std::path::PathBuf;

/// The app's directory layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
}

/// Overrides the base directory. Used by tests and by packaging that wants
/// everything under one tree.
pub const BASE_ENV: &str = "SOSUMI_HOME";

impl Paths {
    /// Resolves the platform layout, honouring `SOSUMI_HOME` when set.
    ///
    /// # Errors
    ///
    /// Fails when neither an override nor the platform's config directory can
    /// be determined (for example, `$HOME` unset on Linux).
    pub fn resolve() -> anyhow::Result<Self> {
        if let Some(base) = std::env::var_os(BASE_ENV) {
            let base = PathBuf::from(base);
            return Ok(Self::under(base));
        }
        let config = dirs_config()?.join("sosumi");
        let data = dirs_data()?.join("sosumi");
        let cache = dirs_cache()?.join("sosumi");
        Ok(Self { config, data, cache })
    }

    /// A layout with every subdirectory under one root.
    #[must_use]
    pub fn under(base: impl Into<PathBuf>) -> Self {
        let base = base.into();
        Self { config: base.join("config"), data: base.join("data"), cache: base.join("cache") }
    }

    /// Settings and credentials.
    #[must_use]
    pub fn config(&self) -> &std::path::Path {
        &self.config
    }

    /// Persistent, user-visible data such as downloads.
    #[must_use]
    pub fn data(&self) -> &std::path::Path {
        &self.data
    }

    /// Disposable data such as cached artwork and segments.
    #[must_use]
    pub fn cache(&self) -> &std::path::Path {
        &self.cache
    }

    /// The SQLite database holding the library index.
    #[must_use]
    pub fn library_db(&self) -> PathBuf {
        self.data.join("library.db")
    }

    /// Downloaded, re-sealed audio files.
    #[must_use]
    pub fn downloads(&self) -> PathBuf {
        self.data.join("downloads")
    }

    /// The encrypted keyring-backed secret store.
    #[must_use]
    pub fn secret_store(&self) -> PathBuf {
        self.config.join("secrets.json")
    }

    /// Session data: cookies and tokens.
    #[must_use]
    pub fn session(&self) -> PathBuf {
        self.config.join("session.json")
    }

    /// User-supplied optional runtime payloads (Apple libraries, Cavern).
    #[must_use]
    pub fn runtime(&self) -> PathBuf {
        self.data.join("runtime")
    }

    /// Creates every directory this layout needs.
    ///
    /// # Errors
    ///
    /// Propagates any filesystem error from directory creation.
    pub fn ensure(&self) -> anyhow::Result<()> {
        for dir in [&self.config, &self.data, &self.cache, &self.runtime()] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

fn dirs_config() -> anyhow::Result<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| {
            anyhow::anyhow!("cannot determine config directory: set $HOME or {BASE_ENV}")
        })
}

fn dirs_data() -> anyhow::Result<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or_else(|| anyhow::anyhow!("cannot determine data directory: set $HOME or {BASE_ENV}"))
}

fn dirs_cache() -> anyhow::Result<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| anyhow::anyhow!("cannot determine cache directory: set $HOME or {BASE_ENV}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_keeps_everything_under_one_root() {
        let paths = Paths::under(PathBuf::from("/tmp/sosumi-test"));
        assert_eq!(paths.config(), std::path::Path::new("/tmp/sosumi-test/config"));
        assert_eq!(paths.data(), std::path::Path::new("/tmp/sosumi-test/data"));
        assert_eq!(paths.downloads(), PathBuf::from("/tmp/sosumi-test/data/downloads"));
    }

    #[test]
    fn operational_paths_are_distinct() {
        let paths = Paths::under(PathBuf::from("/base"));
        assert_ne!(paths.library_db(), paths.secret_store());
        assert_ne!(paths.session(), paths.secret_store());
    }
}
