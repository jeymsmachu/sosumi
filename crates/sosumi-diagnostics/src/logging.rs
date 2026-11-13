//! Logging initialisation.

use std::io::IsTerminal;

use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Environment variable that overrides the default log filter.
pub const LOG_ENV: &str = "SOSUMI_LOG";

/// How chatty the default filter is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Verbosity {
    /// Warnings and errors only.
    Quiet,
    /// Info and above. The default for release builds.
    #[default]
    Normal,
    /// Debug and above.
    Debug,
    /// Everything, including trace.
    Trace,
}

impl Verbosity {
    fn default_filter(self) -> &'static str {
        match self {
            Self::Quiet => "sosumi=warn",
            Self::Normal => "sosumi=info",
            Self::Debug => "sosumi=debug",
            Self::Trace => "sosumi=trace",
        }
    }
}

/// Installs the global tracing subscriber.
///
/// `SOSUMI_LOG` takes precedence when set, so operators can change verbosity
/// without rebuilding.
///
/// # Errors
///
/// Returns an error if a global subscriber has already been installed.
pub fn init(verbosity: Verbosity) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_env(LOG_ENV)
        .unwrap_or_else(|_| EnvFilter::new(verbosity.default_filter()));

    let ansi = std::io::stdout().is_terminal();
    let layer = fmt::layer().with_target(true).with_ansi(ansi);

    tracing_subscriber::registry().with(filter).with(layer).try_init()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_is_less_verbose_than_trace() {
        assert_ne!(Verbosity::Quiet.default_filter(), Verbosity::Trace.default_filter());
    }

    #[test]
    fn normal_is_the_default() {
        assert_eq!(Verbosity::default(), Verbosity::Normal);
    }
}
