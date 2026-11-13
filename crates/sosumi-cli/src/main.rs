//! A command-line harness for exercising the Sosumi core.
//!
//! The GUI is the product, but it is a slow way to answer questions like "does
//! the provider reach Apple" or "does this file decode". This binary drives the
//! same crates directly, so a failure shows up in a terminal with a backtrace
//! instead of as an empty list in a window.
//!
//! It is not shipped and not installed. `publish = false` and it is excluded
//! from packaging; see `packaging/README.md`.

#![forbid(unsafe_code)]

mod commands;
mod context;

use clap::{Parser, Subcommand};

use sosumi_diagnostics::logging::Verbosity;

/// Sosumi's development harness.
#[derive(Debug, Parser)]
#[command(name = "sosumi", version, about)]
struct Cli {
    /// Increase log verbosity. Repeatable: `-vv` is debug, `-vvv` is trace.
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

/// What to do.
#[derive(Debug, Subcommand)]
enum Command {
    /// Show paths, session state, capabilities, and whether audio is available.
    Status,

    /// List the registered providers and whether each is usable.
    Providers,

    /// Search Apple Music.
    Search {
        /// What to search for.
        query: String,
        /// Storefront, for example `us` or `gb`.
        #[arg(long)]
        storefront: Option<String>,
    },

    /// Fetch and print one track's metadata.
    Track {
        /// A track id, either `apple:1440833098` or a bare Apple id.
        id: String,
    },

    /// Fetch and print a track's lyrics.
    Lyrics {
        /// A track id.
        id: String,
    },

    /// Play a track, or a local file, through the audio device.
    Play {
        /// A track id (`apple:…`) or a path to a local audio file.
        target: String,
        /// Play for this many seconds, then stop.
        #[arg(long, default_value_t = 20)]
        seconds: u64,
    },

    /// Check the Widevine device identity used for the baseline tier.
    Identity,

    /// Report which optional capabilities are available on this machine.
    Capabilities,

    /// Render a local audio file through the spatial downmix, to check the path.
    Spatial {
        /// Path to a local audio file.
        path: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let verbosity = match cli.verbose {
        0 => Verbosity::Normal,
        1 => Verbosity::Debug,
        _ => Verbosity::Trace,
    };
    sosumi_diagnostics::logging::init(verbosity).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let runtime =
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;

    runtime.block_on(async {
        let context = context::Context::resolve()?;
        match cli.command {
            // The reporting commands cannot fail; they inspect and print.
            Command::Status => {
                commands::status(&context);
                Ok(())
            }
            Command::Providers => {
                commands::providers(&context);
                Ok(())
            }
            Command::Identity => {
                commands::identity(&context);
                Ok(())
            }
            Command::Capabilities => {
                commands::capabilities(&context);
                Ok(())
            }
            Command::Spatial { path } => commands::spatial(&context, &path),
            Command::Search { query, storefront } => {
                commands::search(&context, &query, storefront.as_deref()).await
            }
            Command::Track { id } => commands::track(&context, &id).await,
            Command::Lyrics { id } => commands::lyrics(&context, &id).await,
            Command::Play { target, seconds } => commands::play(&context, &target, seconds).await,
        }
    })
}
