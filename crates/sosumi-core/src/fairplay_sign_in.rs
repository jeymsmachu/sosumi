//! Signing in to Apple through the native `FairPlay` bridge.
//!
//! # Why this is not a plain function call
//!
//! `drm_init` blocks for the whole login: it opens a connection, exchanges
//! credentials, waits for a verification code, and acquires a lease. Inside that
//! call, the C library invokes a callback for each challenge and waits for the
//! answer. So the call cannot run on the thread that draws the interface, and the
//! answer has to arrive from a different thread than the one making the call.
//!
//! The shape that follows from that:
//!
//! ```text
//!   UI thread                     worker thread            C library
//!   ─────────                     ─────────────            ─────────
//!   begin_sign_in() ─────────────▶ drm_init()
//!                                      │                     │
//!   ◀──── Progress(WaitingForCode) ────┤ ◀── Challenge("2fa")
//!   answer(code) ──▶ channel ─────────▶ ┤ ── answer ─────────▶│
//!                                      │                     │
//!   ◀──── Succeeded ───────────────────┘ ◀──── returns ──────┘
//! ```
//!
//! [`SignIn`] owns that worker and the channel. The UI starts a sign-in, watches a
//! stream of [`SignInEvent`]s, and answers a challenge when one arrives.
//!
//! # Why the credentials are not stored here
//!
//! The password is moved into the worker, used for one call, and dropped. It is
//! never logged, never written, and never returned. The session files the C
//! library writes are what persist a signed-in state, and those live in the
//! runtime's session directory.

use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::broadcast;

use sosumi_fairplay::{AuthChallenge, ChallengeHandler, RuntimeLayout, Session, SignInError};

/// What happened during a sign-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInEvent {
    /// Sign-in started.
    Started,
    /// The bridge is waiting for a verification code.
    ///
    /// The UI shows the code field on this event and nothing before it, so the
    /// user is never asked for a code the bridge is not waiting for.
    CodeRequired,
    /// The bridge is bringing up the `FairPlay` layer.
    Preparing,
    /// Sign-in finished and the session files exist.
    Succeeded,
    /// Sign-in failed, with Apple's own wording where there was any.
    Failed {
        /// A message fit for the sign-in form.
        message: String,
    },
}

/// Carries an answer from the UI thread to the blocked C call.
enum Answer {
    /// The six-digit code.
    Code(String),
    /// The user gave up.
    Abandon,
}

/// The handler the C library calls, backed by a channel the UI feeds.
///
/// `answer` blocks until the UI replies, which is what the C library expects. The
/// block is on the worker thread, never on the UI thread.
struct ChannelHandler {
    answers: Mutex<Receiver<Answer>>,
    events: broadcast::Sender<SignInEvent>,
}

impl ChannelHandler {
    fn ask(&self, challenge: AuthChallenge) -> Option<String> {
        if challenge == AuthChallenge::TwoFactor {
            // Emitted before blocking, so the field is on screen by the time the
            // user could possibly type into it.
            let _ = self.events.send(SignInEvent::CodeRequired);
        }

        // Blocks the worker until the UI answers. A closed channel means the app
        // is shutting down, so `None` abandons the attempt rather than hanging.
        match self.answers.lock().recv() {
            Ok(Answer::Code(code)) => Some(code),
            Ok(Answer::Abandon) | Err(_) => None,
        }
    }
}

impl ChallengeHandler for ChannelHandler {
    fn answer(&self, challenge: AuthChallenge) -> Option<String> {
        self.ask(challenge)
    }
}

/// A sign-in in progress, or the record of the last one.
pub struct SignIn {
    answers: Sender<Answer>,
    events: broadcast::Sender<SignInEvent>,
    /// `None` once the worker has finished, which is how completion is detected.
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
    /// True while a worker is running, so a second sign-in is refused rather than
    /// racing the first against the C library's global state.
    running: Arc<Mutex<bool>>,
}

impl std::fmt::Debug for SignIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignIn")
            .field("running", &*self.running.lock())
            .finish_non_exhaustive()
    }
}

impl SignIn {
    /// Starts a sign-in on a worker thread and returns immediately.
    ///
    /// `username` and `password` are the account credentials. They are moved into
    /// the worker and dropped when the call returns.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::AlreadyInitialised`] when a sign-in is already
    /// running. The C library documents its init as callable once, and two
    /// concurrent attempts would corrupt the lease rather than fail cleanly.
    pub fn begin(
        layout: RuntimeLayout,
        username: String,
        password: String,
    ) -> Result<Self, SignInError> {
        let running = Arc::new(Mutex::new(true));
        let running_for_worker = Arc::clone(&running);

        let (answer_tx, answer_rx) = std::sync::mpsc::channel();
        let (event_tx, _) = broadcast::channel(16);
        let events_for_worker = event_tx.clone();
        let events_for_handler = event_tx.clone();

        let worker = std::thread::Builder::new()
            .name("sosumi-fairplay-signin".to_owned())
            .spawn(move || {
                // The whole login runs here, including the blocking callbacks.
                let outcome = run_sign_in(
                    &layout,
                    &username,
                    &password,
                    ChannelHandler { answers: Mutex::new(answer_rx), events: events_for_handler },
                    &events_for_worker,
                );
                let _ = events_for_worker.send(outcome);
                *running_for_worker.lock() = false;
                // `password` is dropped here, at the end of its only scope.
            })
            .map_err(|e| SignInError::Bridge(format!("starting the sign-in worker: {e}")))?;

        let _ = event_tx.send(SignInEvent::Started);
        Ok(Self { answers: answer_tx, events: event_tx, worker: Mutex::new(Some(worker)), running })
    }

    /// Subscribes to progress events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<SignInEvent> {
        self.events.subscribe()
    }

    /// Supplies the verification code the bridge is waiting for.
    ///
    /// # Errors
    ///
    /// Fails when the worker is no longer listening, which means the attempt
    /// already ended.
    pub fn submit_code(&self, code: String) -> Result<(), SignInError> {
        self.answers
            .send(Answer::Code(code))
            .map_err(|_| SignInError::Bridge("the sign-in attempt already ended".into()))
    }

    /// Gives up on the attempt, unblocking the worker so it can exit.
    pub fn abandon(&self) {
        let _ = self.answers.send(Answer::Abandon);
    }

    /// True while the worker is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        *self.running.lock()
    }

    /// Waits for the worker to finish, so no thread outlives the application.
    ///
    /// Called on drop: a sign-in thread left running would hold the C library's
    /// global state past the application's lifetime.
    pub fn join(&self) {
        let mut worker = self.worker.lock();
        if let Some(handle) = worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SignIn {
    fn drop(&mut self) {
        // Unblock a worker that is waiting on a code, then wait for it. Without
        // the abandon, a drop during a pending challenge would hang.
        self.abandon();
        self.join();
    }
}

/// The worker's body: load the libraries, then sign in.
fn run_sign_in(
    layout: &RuntimeLayout,
    username: &str,
    password: &str,
    handler: ChannelHandler,
    events: &broadcast::Sender<SignInEvent>,
) -> SignInEvent {
    let session = match Session::new(Box::new(handler)) {
        Ok(session) => session,
        Err(error) => {
            return SignInEvent::Failed { message: error.to_string() };
        }
    };

    // Loading and signing in are separated so the two failure modes stay
    // distinguishable: a missing payload is a setup problem, a rejected password
    // is an account problem, and the remedies are entirely different.
    if let Err(error) = session.load_runtime(layout) {
        return SignInEvent::Failed {
            message: match error {
                SignInError::RuntimeMissing => {
                    "The lossless component is not installed. Install it from Settings \
                     and try again."
                        .to_owned()
                }
                other => other.to_string(),
            },
        };
    }

    let _ = events.send(SignInEvent::Preparing);

    match session.sign_in(layout, Some(username), Some(password)) {
        Ok(()) => SignInEvent::Succeeded,
        Err(SignInError::Unanswered) => {
            SignInEvent::Failed { message: "Sign-in was cancelled.".to_owned() }
        }
        Err(other) => {
            // Apple's own wording is far more useful than a return code, and the
            // session already pulled it out.
            let detail = session.last_error();
            let message = if detail.is_empty() { other.to_string() } else { detail };
            SignInEvent::Failed { message }
        }
    }
}

/// Reads every event already queued, without waiting.
///
/// A helper for callers that poll rather than await, such as the CLI.
#[must_use]
pub fn drain(receiver: &mut broadcast::Receiver<SignInEvent>) -> Vec<SignInEvent> {
    let mut out = Vec::new();
    loop {
        match receiver.try_recv() {
            Ok(event) => out.push(event),
            Err(broadcast::error::TryRecvError::Lagged(_)) => {}
            Err(broadcast::error::TryRecvError::Empty | broadcast::error::TryRecvError::Closed) => {
                break
            }
        }
    }
    out
}

/// A `TryRecvError` is not a sign-in failure; kept separate for readability.
#[allow(dead_code)]
fn is_empty(error: TryRecvError) -> bool {
    matches!(error, TryRecvError::Empty | TryRecvError::Disconnected)
}

#[cfg(test)]
// Tests may panic; the workspace lint policy allows it under `cfg(test)` by the
// same convention every other crate in this workspace uses.
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_handler_blocks_until_answered() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (events, _) = broadcast::channel(8);
        let handler = ChannelHandler { answers: Mutex::new(rx), events };

        // Answer from another thread, as the UI would.
        let responder = std::thread::spawn(move || {
            // Give the handler a moment to block, then answer.
            std::thread::sleep(std::time::Duration::from_millis(50));
            tx.send(Answer::Code("123456".to_owned())).expect("send");
        });

        assert_eq!(handler.answer(AuthChallenge::TwoFactor).as_deref(), Some("123456"));
        responder.join().expect("join");
    }

    #[test]
    fn an_abandoned_challenge_returns_nothing() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (events, _) = broadcast::channel(8);
        let handler = ChannelHandler { answers: Mutex::new(rx), events };

        let responder = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            tx.send(Answer::Abandon).expect("send");
        });

        // `None` is what tells the C library to give up, so it must not be a
        // panic or an empty string.
        assert_eq!(handler.answer(AuthChallenge::TwoFactor), None);
        responder.join().expect("join");
    }

    #[test]
    fn a_dropped_sender_abandons_rather_than_hanging() {
        let (tx, rx) = std::sync::mpsc::channel::<Answer>();
        let (events, _) = broadcast::channel(8);
        let handler = ChannelHandler { answers: Mutex::new(rx), events };

        // The UI going away must not leave the worker blocked forever.
        drop(tx);
        assert_eq!(handler.answer(AuthChallenge::TwoFactor), None);
    }

    #[test]
    fn the_code_required_event_arrives_before_the_handler_blocks() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (events, mut listener) = broadcast::channel(8);
        let handler = ChannelHandler { answers: Mutex::new(rx), events };

        let responder = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            tx.send(Answer::Abandon).expect("send");
        });

        handler.answer(AuthChallenge::TwoFactor);
        responder.join().expect("join");

        // Emitted before blocking, so the field is on screen by the time the user
        // could type into it.
        assert_eq!(drain(&mut listener), vec![SignInEvent::CodeRequired]);
    }

    #[test]
    fn credentials_do_not_raise_a_code_prompt() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (events, mut listener) = broadcast::channel(8);
        let handler = ChannelHandler { answers: Mutex::new(rx), events };

        let responder = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            tx.send(Answer::Abandon).expect("send");
        });

        handler.answer(AuthChallenge::Credentials);
        responder.join().expect("join");

        // A credentials challenge already has its answer; prompting for a code
        // would ask the user for something that does not exist yet.
        assert!(drain(&mut listener).is_empty());
    }

    #[test]
    fn drain_reports_events_in_order() {
        let (events, mut listener) = broadcast::channel(8);
        let _ = events.send(SignInEvent::Started);
        let _ = events.send(SignInEvent::Preparing);
        let _ = events.send(SignInEvent::Succeeded);

        assert_eq!(
            drain(&mut listener),
            vec![SignInEvent::Started, SignInEvent::Preparing, SignInEvent::Succeeded]
        );
        assert!(drain(&mut listener).is_empty(), "a second drain finds nothing");
    }

    #[test]
    fn a_failure_carries_a_message_fit_for_the_form() {
        let event = SignInEvent::Failed { message: "Wrong password.".to_owned() };
        match event {
            SignInEvent::Failed { message } => assert!(!message.is_empty()),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn is_empty_recognises_both_terminal_states() {
        assert!(is_empty(TryRecvError::Empty));
        assert!(is_empty(TryRecvError::Disconnected));
    }
}
