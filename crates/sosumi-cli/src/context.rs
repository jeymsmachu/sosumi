//! The shared state every command needs.
//!
//! Responsibilities are deliberately identical to the bridge's composition
//! root: resolve paths, build the core, register the providers, start audio.
//! Keeping that in one place means a CLI command and the GUI exercise the same
//! wiring, so a bug found here is a real bug.

use std::sync::Arc;

use sosumi_core::Sosumi;
use sosumi_provider_apple::AppleMusicProvider;
use sosumi_provider_local::LocalProvider;
use sosumi_storage::Paths;

/// A built application plus the pieces a command may want to inspect.
pub(crate) struct Context {
    /// The core application.
    pub app: Arc<Sosumi>,
    /// The Apple provider, for commands that need it directly.
    pub apple: Arc<AppleMusicProvider>,
    /// The resolved storage layout.
    pub paths: Paths,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").field("paths", &self.paths).finish_non_exhaustive()
    }
}

impl Context {
    /// Builds the application the same way the GUI does.
    ///
    /// # Errors
    ///
    /// Fails when the storage layout cannot be created, the HTTP client cannot
    /// be built, or the runtime is unavailable.
    pub(crate) fn resolve() -> anyhow::Result<Self> {
        let paths = Paths::resolve()?;
        paths.ensure()?;

        let mut app = Sosumi::new(paths.clone())?;

        let apple = Arc::new(AppleMusicProvider::new().map_err(|e| anyhow::anyhow!("{e}"))?);
        app.register(Arc::clone(&apple) as Arc<dyn sosumi_provider_api::MediaProvider>);

        let music_dirs = default_music_directories();
        if !music_dirs.is_empty() {
            app.register(Arc::new(LocalProvider::new(music_dirs))
                as Arc<dyn sosumi_provider_api::MediaProvider>);
        }

        Ok(Self { app: Arc::new(app), apple, paths })
    }

    /// Starts the audio engine and reports whether it came up.
    pub(crate) fn start_audio(&self) {
        self.app.start_audio();
    }
}

/// The directories to scan for local music.
fn default_music_directories() -> Vec<std::path::PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = std::path::PathBuf::from(home);
    let music = std::env::var_os("XDG_MUSIC_DIR")
        .map_or_else(|| home.join("Music"), std::path::PathBuf::from);

    if music.is_dir() {
        vec![music]
    } else {
        Vec::new()
    }
}
