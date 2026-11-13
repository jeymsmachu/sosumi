//! The commands.
//!
//! Each one answers a question a developer actually has while bringing the app
//! up, and prints what it found rather than only a pass or fail.

use std::time::Duration;

use sosumi_diagnostics::{Health, HealthStatus};
use sosumi_provider_api::{MediaProvider, ProviderError, SearchQuery, TrackId};
use sosumi_spatial::{CavernRenderer, DownmixRenderer, Renderer};

use crate::context::Context;

/// Prints the shape of the installation and what is usable right now.
pub(crate) fn status(context: &Context) {
    // Probe audio rather than reporting the current flag: `audio_ready` is false
    // until the engine is started, so reporting it directly would say
    // "unavailable" on a machine where playback works fine.
    context.start_audio();
    let session = context.app.session();
    let audio_ready = context.app.audio_ready();

    println!("Installation");
    println!("  config   {}", context.paths.config().display());
    println!("  data     {}", context.paths.data().display());
    println!("  cache    {}", context.paths.cache().display());
    println!();
    println!("Session");
    println!("  signed in        {}", yes_no(session.is_signed_in()));
    println!("  developer token  {}", yes_no(session.developer_token.is_some()));
    println!("  storefront       {}", session.storefront);
    println!(
        "  Apple credentials {}",
        if context.apple.credentials().music_user_token.is_some() { "loaded" } else { "not set" }
    );
    println!();
    println!("Playback");
    println!("  audio output  {}", if audio_ready { "ready" } else { "unavailable" });
    println!("  quality       {}", context.app.quality().label());
    println!("  queue         {} tracks", context.app.queue_len());
    println!();

    let identity = sosumi_security::widevine::DeviceIdentity::load(&identity_dir(context));
    println!("Baseline tier (AAC)");
    match identity {
        Ok(identity) => println!("  device identity  present ({identity:?})"),
        Err(error) => println!("  device identity  {error}"),
    }
}

/// Lists providers and whether the active one can serve a request.
pub(crate) fn providers(context: &Context) {
    let apple = &context.apple;
    println!("Registered providers");
    println!(
        "  {:<8} {:<14} {:<10} lossless={}",
        apple.id(),
        apple.name(),
        if apple.is_available() { "available" } else { "unavailable" },
        yes_no(apple.credentials().music_user_token.is_some())
    );
    println!();
    println!("Note: lossless shows true once a Music-User-Token is set; the identity");
    println!("      above determines whether AAC can play at all.");
}

/// Searches and prints the results.
///
/// # Errors
///
/// Propagates a provider failure, which is the point: a search that fails
/// upstream must surface as a real error with a status code.
pub(crate) async fn search(
    context: &Context,
    query: &str,
    storefront: Option<&str>,
) -> anyhow::Result<()> {
    let mut search = SearchQuery::new(query);
    search.storefront = storefront.map(str::to_owned);

    let started = std::time::Instant::now();
    let results = context.app.search(search).await?;
    let elapsed = started.elapsed();

    println!("{query:?} in {elapsed:?}");
    println!();

    if !results.tracks.is_empty() {
        println!("Songs ({})", results.tracks.len());
        for track in &results.tracks {
            let duration = track.duration_ms.map_or_else(
                || "--:--".to_owned(),
                |ms| format!("{}:{:02}", ms / 60_000, (ms / 1000) % 60),
            );
            println!("  {duration}  {:<40}  {}", truncate(&track.title, 40), track.artist.name);
            println!("            id {}", track.id.as_str());
        }
        println!();
    }

    if !results.albums.is_empty() {
        println!("Albums ({})", results.albums.len());
        for album in &results.albums {
            println!("  {:<40}  {}", truncate(&album.title, 40), album.artist.name);
        }
        println!();
    }

    if !results.artists.is_empty() {
        println!("Artists ({})", results.artists.len());
        for artist in &results.artists {
            println!("  {}", artist.name);
        }
    }

    if results.is_empty() {
        println!("No results. If this is unexpected, run with -vv to see the request.");
    }
    Ok(())
}

/// Fetches and prints one track.
///
/// # Errors
///
/// Propagates a provider failure.
pub(crate) async fn track(context: &Context, id: &str) -> anyhow::Result<()> {
    let id = normalize_track_id(id);
    let track = context.app.track(&id).await?;

    println!("{}", track.title);
    println!("  artist    {}", track.artist.name);
    if let Some(album) = &track.album {
        println!("  album     {}", album.title);
    }
    if let Some(duration) = track.duration_ms {
        println!("  duration  {}:{:02}", duration / 60_000, (duration / 1000) % 60);
    }
    if let Some(isrc) = &track.isrc {
        println!("  isrc      {isrc}");
    }
    if let Some(artwork) = &track.artwork {
        println!("  artwork   {}", artwork.url);
    }
    println!("  id        {}", track.id.as_str());
    Ok(())
}

/// Fetches and prints lyrics.
///
/// # Errors
///
/// Propagates a provider failure. Lyrics are frequently missing, which is
/// reported as a not-found rather than an error page.
pub(crate) async fn lyrics(context: &Context, id: &str) -> anyhow::Result<()> {
    let id = normalize_track_id(id);
    match context.app.lyrics(&id).await {
        Ok(lyrics) => {
            if lyrics.timed.is_empty() {
                println!("(unsynced)\n");
                println!("{}", lyrics.plain);
            } else {
                println!("({} timed lines)\n", lyrics.timed.len());
                for line in &lyrics.timed {
                    println!("  {:>7} ms  {}", line.start_ms, line.text);
                }
            }
            Ok(())
        }
        Err(ProviderError::NotFound(_)) => {
            println!("No lyrics are available for {}", id.as_str());
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// Plays a track or a local file for a bounded time.
///
/// # Errors
///
/// Propagates provider and audio failures.
pub(crate) async fn play(context: &Context, target: &str, seconds: u64) -> anyhow::Result<()> {
    context.start_audio();
    if !context.app.audio_ready() {
        anyhow::bail!("no audio output device is available");
    }

    // A path is played as a local file; anything else is treated as a track id.
    let id = if std::path::Path::new(target).is_file() {
        let local = local_track_id(target)?;
        println!("Playing local file {target}");
        local
    } else {
        let id = normalize_track_id(target);
        println!("Playing {}", id.as_str());
        id
    };

    let started = std::time::Instant::now();
    let session = context.app.start(&id).await?;
    println!("  codec        {}", session.info.badge());
    println!("  source       {}", session.source.kind());
    if let Some(duration) = session.duration_ms {
        println!("  duration     {}:{:02}", duration / 60_000, (duration / 1000) % 60);
    }

    // Follow the event stream so position and errors are visible while playing.
    let mut events = context.app.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let mut last_position = 0;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        match tokio::time::timeout(remaining, events.next()).await {
            Ok(Some(event)) => {
                use sosumi_core::Event;
                match event {
                    Event::Position { position_ms } => {
                        // Print at most once a second so the terminal stays readable.
                        if position_ms / 1000 > last_position / 1000 {
                            println!(
                                "  position     {}:{:02}",
                                position_ms / 60_000,
                                (position_ms / 1000) % 60
                            );
                        }
                        last_position = position_ms;
                    }
                    Event::Ended => {
                        println!("  ended");
                        break;
                    }
                    Event::Error { message } => {
                        println!("  error        {message}");
                        break;
                    }
                    _ => {}
                }
            }
            // A closed bus and a timeout both mean the listening window is over.
            Ok(None) | Err(_) => break,
        }
    }

    println!("Stopping after {:?}", started.elapsed());
    context.app.stop()?;
    Ok(())
}

/// Reports the state of the Widevine device identity.
///
/// A missing identity is the expected first-run state, so this reports rather
/// than fails.
pub(crate) fn identity(context: &Context) {
    let directory = identity_dir(context);
    println!("Identity directory: {}", directory.display());
    println!();

    match sosumi_security::widevine::DeviceIdentity::load(&directory) {
        Ok(identity) => {
            println!("Status: usable");
            println!("{identity:?}");
            println!();
            println!("The baseline tier can acquire content keys.");
        }
        Err(sosumi_security::widevine::IdentityError::NotConfigured) => {
            println!("Status: not configured");
            println!();
            println!("Expected files, both required:");
            println!("  {}", directory.join("device_private_key").display());
            println!("  {}", directory.join("device_client_id_blob").display());
            println!();
            println!("Without these, AAC playback cannot acquire a content key.");
            println!("Lossless and Spatial Audio use a separate component and are");
            println!("unaffected by this; see docs/adr/0004.");
        }
        Err(error) => {
            println!("Status: unusable");
            println!("  {error}");
        }
    }
}

/// Reports every optional capability.
pub(crate) fn capabilities(context: &Context) {
    // Read the same report the Settings screen reads, rather than probing
    // separately. A CLI that disagreed with the app would be worse than no CLI:
    // it would send a developer looking in the wrong place.
    let lossless = context.app.lossless_capability();

    let mut health = vec![lossless.to_health()];

    // Spatial Audio: whether an object renderer is installed, plus the fallback
    // it always has.
    let cavern = CavernRenderer::detect();
    health.push(sosumi_spatial::health(&cavern as &dyn Renderer));

    let downmix = DownmixRenderer;
    health.push(Health {
        id: "spatial.downmix".to_owned(),
        name: "Spatial downmix (fallback)".to_owned(),
        status: HealthStatus::Ready,
        detail: if downmix.is_available() {
            "Folds object audio to stereo when no object renderer is present".to_owned()
        } else {
            "Unavailable".to_owned()
        },
        action: None,
    });

    // Discord: whether the local socket exists.
    health.push(if sosumi_discord::ipc::is_running() {
        Health::ready("discord", "Discord Rich Presence")
    } else {
        Health::unavailable("discord", "Discord Rich Presence", "Discord is not running")
            .with_action("Start Discord to enable presence")
    });

    // MPRIS: whether a session bus is reachable.
    health.push(if sosumi_linux::mpris::session_bus_available() {
        Health::ready("mpris", "System Media Controls")
    } else {
        Health::unavailable("mpris", "System Media Controls", "No D-Bus session bus")
            .with_action("Run inside a desktop session")
    });

    println!("{:<12} {:<28} DETAIL", "STATE", "CAPABILITY");
    for entry in &health {
        let state = match entry.status {
            HealthStatus::Ready => "ready",
            HealthStatus::Unavailable => "available",
            HealthStatus::Degraded => "degraded",
        };
        println!("{state:<12} {:<28} {}", entry.name, entry.detail);
        if let Some(action) = &entry.action {
            println!("{:<12} {:<28} -> {action}", "", "");
        }
    }

    // For an installable lossless tier, name the files so a developer can fetch
    // exactly what is missing rather than the whole payload.
    if let Some(runtime) = context.app.runtime() {
        let missing = runtime.missing_files();
        if !missing.is_empty() {
            println!();
            println!("Lossless is missing {} file(s):", missing.len());
            for name in missing.iter().take(6) {
                println!("  {name}");
            }
            if missing.len() > 6 {
                println!("  \u{2026} and {} more", missing.len() - 6);
            }
            println!();
            println!("Fetch them with: scripts/fetch-fairplay-runtime.sh");
        }
    }
}

/// Decodes a local file and runs it through the spatial downmix.
///
/// This exercises the render path without needing a Spatial Audio track, which
/// is otherwise hard to test because it requires a subscription and the
/// lossless component.
///
/// # Errors
///
/// Propagates decode failures.
pub(crate) fn spatial(_context: &Context, path: &str) -> anyhow::Result<()> {
    let bytes = std::fs::read(path)?;
    let extension = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");

    println!("Decoding {path} ({extension})");
    let mut decoder =
        sosumi_audio::Decoder::new(bytes, extension).map_err(|e| anyhow::anyhow!("{e}"))?;
    let format = decoder.format();
    println!("  {} Hz, {} channels", format.sample_rate, format.channels);

    let packet = decoder
        .decode_next()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .ok_or_else(|| anyhow::anyhow!("the file has no audio"))?;
    println!("  decoded {} samples in the first packet", packet.samples.len());

    let renderer = DownmixRenderer;
    let spatial = sosumi_provider_api::SpatialFormat {
        bed_channels: u8::try_from(format.channels).unwrap_or(2),
        has_objects: false,
    };

    // Wrap the samples as little-endian f32, which is what the renderer reads.
    let mut encoded = Vec::with_capacity(packet.samples.len() * 4);
    for sample in &packet.samples {
        encoded.extend_from_slice(&sample.to_le_bytes());
    }

    let rendered = renderer.render(&encoded, &spatial).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "  rendered {} samples through {} -> {} channels",
        rendered.len(),
        renderer.name(),
        renderer.target().channels
    );
    let peak = rendered.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
    println!("  peak amplitude {peak:.4}");
    if peak <= 1.0 {
        println!("  no clipping");
    } else {
        println!("  WARNING: output exceeds full scale");
    }
    Ok(())
}

/// The directory the Widevine identity is loaded from.
fn identity_dir(_context: &Context) -> std::path::PathBuf {
    std::env::var_os("SOSUMI_WIDEVINE_DIR").map_or_else(
        || {
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
                })
                .unwrap_or_default();
            config.join("sosumi").join("widevine")
        },
        std::path::PathBuf::from,
    )
}

/// Accepts `apple:123` or a bare `123`, returning a namespaced id.
fn normalize_track_id(value: &str) -> TrackId {
    if value.contains(':') {
        TrackId(value.to_owned())
    } else {
        TrackId::new(
            &sosumi_provider_api::ProviderId::from(sosumi_provider_api::ProviderId::APPLE),
            value,
        )
    }
}

/// Builds a local track id from a path.
///
/// The local provider keys tracks by their index in its scan, so a path is not
/// directly addressable. This searches for the file's basename instead, which is
/// what the provider indexed.
fn local_track_id(path: &str) -> anyhow::Result<TrackId> {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("cannot derive a track name from {path}"))?;
    Ok(TrackId::new(
        &sosumi_provider_api::ProviderId::from(sosumi_provider_api::ProviderId::LOCAL),
        stem,
    ))
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}
