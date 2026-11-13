//! Fetching the optional runtime payload.
//!
//! Apple's libraries are not distributed with Sosumi. They are downloaded to the
//! user's own machine, only when the user asks for the lossless tier, and every
//! file is verified against its pinned SHA-256 before it is used.
//!
//! This is the in-app form of `scripts/fetch-fairplay-runtime.sh`, with the one
//! difference the interface needs: it reports progress as it goes, so an install
//! can show a determinate bar rather than an indeterminate spinner. `loading.md ›
//! Showing progress`: "you use a *determinate* progress indicator when you know
//! how long loading will take".
//!
//! # Why the network lives here and not in `sosumi-fairplay`
//!
//! The payload crate stays free of HTTP so its install path is testable without a
//! connection, and so the crate that wraps the foreign libraries stays small.
//! Fetching is orchestration, which is this crate's job.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt as _;
use sha2::{Digest as _, Sha256};
use sosumi_fairplay::PinnedFile;

/// Identifies the client to the pinned mirrors.
const USER_AGENT: &str = "Sosumi/0.1 (unofficial Apple Music client)";

/// How many bytes may arrive between progress reports.
///
/// Reporting every chunk would flood the channel for no visual gain: a 10 MiB
/// file arrives in thousands of chunks but needs only a few hundred samples to
/// look smooth.
const PROGRESS_INTERVAL: u64 = 512 * 1024;

/// The longest a single file may take before the transfer is abandoned.
const PER_FILE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// The longest a single read may stall before the transfer is abandoned.
///
/// A total timeout alone cannot tell a slow-but-working connection from a dead
/// one, and a download that hangs with no explanation is worse than a retry.
const STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// What an install is doing, for the progress UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallEvent {
    /// Beginning. `files` is how many need fetching, which makes the bar
    /// determinate from the first frame even before any byte arrives.
    Started {
        /// How many files will be fetched.
        files: usize,
    },
    /// A file is downloading. `index` is one-based.
    Progress {
        /// The file being fetched.
        file: String,
        /// One-based position in the queue.
        index: usize,
        /// How many files are queued.
        files: usize,
        /// Bytes of this file received so far.
        done: u64,
        /// This file's size, when the server reported it.
        total: Option<u64>,
    },
    /// A file has arrived and its checksum is being compared.
    Verifying {
        /// The file being checked.
        file: String,
        /// One-based position in the queue.
        index: usize,
        /// How many files are queued.
        files: usize,
    },
    /// The install finished. `fetched` is how many files were downloaded, which is
    /// zero when everything was already present.
    Finished {
        /// Files downloaded during this run.
        fetched: usize,
    },
    /// The user stopped it. Files already completed are kept.
    Cancelled,
    /// A file could not be obtained. `message` says which and why.
    Failed {
        /// A short description fit for the screen.
        message: String,
    },
}

/// Why one download attempt failed.
enum SourceError {
    /// The user cancelled.
    Cancelled,
    /// The source could not supply a file matching the pin.
    Failed(String),
}

/// Fetches every pinned file that is missing or wrong, reporting progress.
///
/// A file already present with the right hash is left alone, so a repair run
/// after a partial download fetches only what is absent. Returns how many files
/// were downloaded.
///
/// Takes the payload directory and the pinned list by value so the whole call is
/// `'static` and can run on a spawned task.
///
/// # Errors
///
/// Fails when a file cannot be obtained from any of its pinned sources. Nothing
/// partial is left behind: a download that fails its checksum is deleted rather
/// than kept.
pub async fn fetch_runtime(
    lib64: PathBuf,
    files: Vec<PinnedFile>,
    cancel: Arc<AtomicBool>,
    report: impl Fn(InstallEvent) + Send,
) -> anyhow::Result<usize> {
    std::fs::create_dir_all(&lib64)
        .map_err(|e| anyhow::anyhow!("creating {}: {e}", lib64.display()))?;

    let wanted: Vec<PinnedFile> = files.into_iter().filter(|f| !is_current(&lib64, f)).collect();

    if wanted.is_empty() {
        // The common case after a successful install: everything verifies, so
        // there is nothing to download and no reason to open a connection.
        report(InstallEvent::Finished { fetched: 0 });
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(PER_FILE_TIMEOUT)
        .build()
        .map_err(|e| anyhow::anyhow!("building the download client: {e}"))?;

    let files_total = wanted.len();
    report(InstallEvent::Started { files: files_total });

    let mut fetched = 0;
    for (offset, file) in wanted.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            report(InstallEvent::Cancelled);
            return Ok(fetched);
        }

        let index = offset + 1;
        match download(&client, &lib64, file, index, files_total, &cancel, &report).await {
            Ok(()) => fetched += 1,
            Err(SourceError::Cancelled) => {
                report(InstallEvent::Cancelled);
                return Ok(fetched);
            }
            Err(SourceError::Failed(message)) => {
                return Err(anyhow::anyhow!("{}: {message}", file.name));
            }
        }
    }

    report(InstallEvent::Finished { fetched });
    Ok(fetched)
}

/// Fetches one file, trying each pinned source in turn.
async fn download(
    client: &reqwest::Client,
    lib64: &Path,
    file: &PinnedFile,
    index: usize,
    files: usize,
    cancel: &AtomicBool,
    report: &(impl Fn(InstallEvent) + Send),
) -> Result<(), SourceError> {
    let mut last = "no source was reachable".to_owned();

    for source in &file.sources {
        match try_source(client, lib64, file, index, files, cancel, report, source).await {
            Ok(()) => return Ok(()),
            Err(SourceError::Cancelled) => return Err(SourceError::Cancelled),
            Err(SourceError::Failed(message)) => {
                // A source that fails is not fatal while another remains: the
                // pins name two mirrors precisely so one being unreachable is a
                // retry rather than an error.
                tracing::warn!(
                    target: "sosumi::install",
                    file = %file.name,
                    source,
                    %message,
                    "pinned source failed"
                );
                last = message;
            }
        }
    }

    Err(SourceError::Failed(last))
}

/// Streams one file to disk from one source, verifying the pin as it arrives.
#[allow(clippy::too_many_arguments)]
async fn try_source(
    client: &reqwest::Client,
    lib64: &Path,
    file: &PinnedFile,
    index: usize,
    files: usize,
    cancel: &AtomicBool,
    report: &(impl Fn(InstallEvent) + Send),
    source: &str,
) -> Result<(), SourceError> {
    let response = client
        .get(source)
        .send()
        .await
        .map_err(|e| SourceError::Failed(format!("request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(SourceError::Failed(format!("server returned {}", response.status())));
    }

    let total = response.content_length();

    // Written beside its destination so the final rename is atomic: a reader
    // never sees a half-written library, and a crash leaves a dotfile rather
    // than a file that looks installed and is not.
    let destination = lib64.join(&file.name);
    let partial = lib64.join(format!(".{}.partial", file.name));

    let mut hasher = Sha256::new();
    let mut written: u64 = 0;
    let mut reported: u64 = 0;
    let mut stream = response.bytes_stream();

    report(InstallEvent::Progress { file: file.name.clone(), index, files, done: 0, total });

    // Truncate any leftover from an earlier attempt before appending.
    let _ = std::fs::remove_file(&partial);

    while let Some(chunk) = tokio::time::timeout(STALL_TIMEOUT, stream.next())
        .await
        .map_err(|_| SourceError::Failed("the transfer stalled".to_owned()))?
    {
        if cancel.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&partial);
            return Err(SourceError::Cancelled);
        }

        let chunk = chunk.map_err(|e| SourceError::Failed(format!("reading the response: {e}")))?;

        {
            use std::io::Write as _;
            hasher.update(&chunk);
            let mut handle =
                std::fs::OpenOptions::new().create(true).append(true).open(&partial).map_err(
                    |e| SourceError::Failed(format!("writing {}: {e}", partial.display())),
                )?;
            handle
                .write_all(&chunk)
                .map_err(|e| SourceError::Failed(format!("writing {}: {e}", partial.display())))?;
        }

        written += chunk.len() as u64;
        if written - reported >= PROGRESS_INTERVAL {
            reported = written;
            report(InstallEvent::Progress {
                file: file.name.clone(),
                index,
                files,
                done: written,
                total,
            });
        }
    }

    report(InstallEvent::Verifying { file: file.name.clone(), index, files });

    let actual = hex::encode(hasher.finalize());
    if !actual.eq_ignore_ascii_case(&file.sha256) {
        // Refused, not warned about: the loader would otherwise `dlopen` whatever
        // is on disk, and that code runs with the process's full privileges.
        let _ = std::fs::remove_file(&partial);
        return Err(SourceError::Failed(format!(
            "checksum mismatch (expected {}, got {actual})",
            file.sha256
        )));
    }

    std::fs::rename(&partial, &destination)
        .map_err(|e| SourceError::Failed(format!("installing {}: {e}", destination.display())))?;

    report(InstallEvent::Progress {
        file: file.name.clone(),
        index,
        files,
        done: written,
        total: Some(written),
    });

    Ok(())
}

/// True when the file is present and hashes to its pin.
fn is_current(lib64: &Path, file: &PinnedFile) -> bool {
    std::fs::read(lib64.join(&file.name))
        .is_ok_and(|bytes| hex::encode(Sha256::digest(&bytes)).eq_ignore_ascii_case(&file.sha256))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn pinned(name: &str, sha256: &str, sources: &[&str]) -> PinnedFile {
        PinnedFile {
            name: name.to_owned(),
            sha256: sha256.to_owned(),
            sources: sources.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// The SHA-256 of `b"correct"`, computed here so the test does not depend on
    /// a value copied by hand.
    fn hash_of(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    #[test]
    fn a_present_file_with_the_right_hash_needs_no_fetch() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("lib.sh"), b"correct").expect("write");
        let file = pinned("lib.sh", &hash_of(b"correct"), &["http://127.0.0.1:1/never"]);

        assert!(is_current(dir.path(), &file));
    }

    #[test]
    fn an_altered_file_is_not_current() {
        // The case a hash exists to catch: a file that is present, plausible, and
        // wrong. Treating it as installed would let the loader run on it.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("lib.sh"), b"tampered").expect("write");
        let file = pinned("lib.sh", &hash_of(b"correct"), &[]);

        assert!(!is_current(dir.path(), &file));
    }

    #[test]
    fn a_missing_file_is_not_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = pinned("absent.so", &hash_of(b"x"), &[]);

        assert!(!is_current(dir.path(), &file));
    }

    #[test]
    fn a_payload_that_is_already_complete_fetches_nothing() {
        // No network is opened: an install over a complete payload must be
        // instant and offline.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("lib.sh"), b"correct").expect("write");
        let file = pinned("lib.sh", &hash_of(b"correct"), &["http://127.0.0.1:1/never"]);

        let events = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&events);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

        let fetched = runtime
            .block_on(fetch_runtime(
                dir.path().to_path_buf(),
                vec![file],
                Arc::new(AtomicBool::new(false)),
                move |event| sink.lock().push(event),
            ))
            .expect("fetch");

        assert_eq!(fetched, 0);
        assert_eq!(*events.lock(), vec![InstallEvent::Finished { fetched: 0 }]);
    }

    /// A source that serves one body, so the fetch path can be exercised without
    /// the real network.
    fn serve_once(body: &'static [u8]) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = stream.read(&mut buffer);
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body);
                let _ = stream.flush();
            }
        });
        (format!("http://{addr}/file"), handle)
    }

    #[test]
    fn a_file_is_fetched_verified_and_installed() {
        // The whole point of the installer: bytes arrive, the pin is checked, and
        // only then does the file appear under its real name.
        let payload = b"the real bytes";
        let (url, server) = serve_once(payload);

        let dir = tempfile::tempdir().expect("tempdir");
        let file = pinned("libReal.so", &hash_of(payload), &[&url]);

        let events = std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&events);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

        let fetched = runtime
            .block_on(fetch_runtime(
                dir.path().to_path_buf(),
                vec![file],
                Arc::new(AtomicBool::new(false)),
                move |event| sink.lock().push(event),
            ))
            .expect("fetch");

        server.join().expect("server");
        assert_eq!(fetched, 1);
        assert_eq!(std::fs::read(dir.path().join("libReal.so")).expect("read"), payload);

        let seen = events.lock().clone();
        assert!(matches!(seen.first(), Some(InstallEvent::Started { files: 1 })));
        assert!(matches!(seen.last(), Some(InstallEvent::Finished { fetched: 1 })));
        assert!(
            seen.iter().any(|e| matches!(e, InstallEvent::Verifying { .. })),
            "verification must be observable, not silent: {seen:?}"
        );
        // No partial file survives a successful install.
        assert!(!dir.path().join(".libReal.so.partial").exists());
    }

    #[test]
    fn a_file_that_does_not_match_its_pin_is_refused_and_deleted() {
        // The security property, not a nicety: the loader would otherwise dlopen
        // whatever is on disk with the process's full privileges.
        let (url, server) = serve_once(b"tampered");

        let dir = tempfile::tempdir().expect("tempdir");
        let file = pinned("libReal.so", &hash_of(b"the real bytes"), &[&url]);

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let result = runtime.block_on(fetch_runtime(
            dir.path().to_path_buf(),
            vec![file],
            Arc::new(AtomicBool::new(false)),
            |_| {},
        ));

        server.join().expect("server");
        assert!(result.is_err(), "a mismatched file must fail the install");
        assert!(!dir.path().join("libReal.so").exists(), "nothing may be installed");
        assert!(
            !dir.path().join(".libReal.so.partial").exists(),
            "the rejected download must be removed, not left beside the others"
        );
    }

    #[test]
    fn a_cancelled_install_stops_and_keeps_what_it_finished() {
        // Two files and no sources: if the cancel flag were ignored, the first
        // fetch would be attempted and fail, so reaching `Ok(0)` proves the loop
        // stopped before any network work.
        let dir = tempfile::tempdir().expect("tempdir");
        let files = vec![pinned("a.so", "00", &[]), pinned("b.so", "11", &[])];

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let fetched = runtime
            .block_on(fetch_runtime(
                dir.path().to_path_buf(),
                files,
                Arc::new(AtomicBool::new(true)),
                |_| {},
            ))
            .expect("cancel is not an error");

        assert_eq!(fetched, 0);
    }

    #[test]
    fn a_cancel_after_some_files_keeps_them() {
        // The property that makes cancel useful: a stop mid-way is resumable, so
        // the next attempt fetches only what is left.
        let payload = b"the real bytes";
        let (url, server) = serve_once(payload);

        let dir = tempfile::tempdir().expect("tempdir");
        let files = vec![pinned("a.so", &hash_of(payload), &[&url]), pinned("b.so", "11", &[])];

        // Cancelled after the first file is written but before the second.
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

        let fetched = runtime
            .block_on(fetch_runtime(
                dir.path().to_path_buf(),
                files,
                Arc::clone(&cancel),
                move |event| {
                    if matches!(event, InstallEvent::Verifying { .. }) {
                        flag.store(true, Ordering::Relaxed);
                    }
                },
            ))
            .expect("cancel is not an error");

        server.join().expect("server");
        assert_eq!(fetched, 1, "the finished file counts");
        assert!(dir.path().join("a.so").is_file(), "a verified file is kept");
        assert!(!dir.path().join("b.so").exists());
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod network_tests {
    //! Exercises the real pinned source. Ignored by default so the suite stays
    //! offline and fast; run with
    //! `cargo test -p sosumi-core -- --ignored real_pinned_source`.

    use super::*;

    #[test]
    #[ignore = "needs the network"]
    fn real_pinned_source_fetches_and_verifies() {
        // The smallest pinned file, so the check is quick. Its hash comes from
        // the generated manifest, which is the same value the app would use.
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../apps/desktop/assets/runtime.toml"),
        )
        .expect("the generated manifest");

        let parsed = sosumi_fairplay::RuntimeManifest::from_toml(&manifest).expect("parse");
        let file = parsed
            .files
            .iter()
            .find(|f| f.name == "libdaapkit.so")
            .expect("libdaapkit in the manifest")
            .clone();

        let dir = tempfile::tempdir().expect("tempdir");
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

        let fetched = runtime
            .block_on(fetch_runtime(
                dir.path().to_path_buf(),
                vec![file.clone()],
                Arc::new(AtomicBool::new(false)),
                |_| {},
            ))
            .expect("fetch from the pinned source");

        assert_eq!(fetched, 1);
        let written = std::fs::read(dir.path().join(&file.name)).expect("read back");
        assert_eq!(
            hex::encode(Sha256::digest(&written)),
            file.sha256,
            "the real bytes must match the pin"
        );
    }
}
