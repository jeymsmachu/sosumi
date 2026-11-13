//! The safe face of the `FairPlay` bridge.
//!
//! Everything the application needs, and nothing about C. The foreign calls and
//! their pointer handling live in [`super::ffi`]; this module turns them into
//! something with a type, a lifetime, and an error.
//!
//! # Why this is a singleton
//!
//! The C library holds global state and documents `drm_lib_init` as callable
//! exactly once per process. A second call does not return an error: it blocks or
//! corrupts the lease. [`Session`] therefore refuses to be created twice, so the
//! mistake is caught in Rust rather than in a C library that cannot report it.
//!
//! `drm_lib_abort_init` exists for the case where the first attempt hung. A
//! failed attempt releases the guard, so the user can retry without restarting
//! the application — the alternative is having to quit an app because a sign-in
//! timed out.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use crate::ffi;
use crate::runtime::RuntimeLayout;

/// What the C library is asking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthChallenge {
    /// An Apple ID and password are needed.
    Credentials,
    /// The six-digit code Apple sent to a trusted device.
    TwoFactor,
}

impl AuthChallenge {
    /// The string the C library sends for this challenge.
    #[must_use]
    const fn as_c_str(self) -> &'static str {
        match self {
            Self::Credentials => "credentials",
            Self::TwoFactor => "2fa",
        }
    }
}

/// What the bridge is doing, as reported by its state callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// The bridge is starting up.
    Starting,
    /// Signing in.
    LoggingIn,
    /// Waiting for a verification code.
    WaitingForCode,
    /// Running and able to serve keys.
    Running,
    /// Bringing up the `FairPlay` layer.
    Preparing,
    /// The last attempt failed.
    Failed,
    /// A state this build does not know.
    Unknown(String),
}

impl SessionState {
    fn from_c_str(value: &str) -> Self {
        match value {
            "STARTING" => Self::Starting,
            "LOGIN" => Self::LoggingIn,
            "WAITING_2FA" => Self::WaitingForCode,
            "RUNNING" => Self::Running,
            "INITIALIZING_FAIRPLAY" => Self::Preparing,
            "FAILED" => Self::Failed,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

/// Errors from sign-in.
#[derive(Debug, thiserror::Error)]
pub enum SignInError {
    /// The user supplied nothing for a challenge that requires an answer.
    #[error("the sign-in challenge was not answered")]
    Unanswered,
    /// A string could not be passed across the boundary, which means it held a
    /// nul byte.
    #[error("a value contained a nul byte and cannot cross the C boundary")]
    InteriorNul,
    /// The payload is missing or damaged.
    #[error("the `FairPlay` runtime is not installed")]
    RuntimeMissing,
    /// A second session was attempted in one process.
    #[error("a `FairPlay` session already exists in this process")]
    AlreadyInitialised,
    /// The bridge reported failure.
    #[error("{0}")]
    Bridge(String),
}

/// Answers a challenge from the bridge.
///
/// The bridge calls this synchronously and blocks until it returns, so an
/// implementation must not wait on the UI thread. In this application it is
/// served from a channel the sign-in screen feeds, which is why the trait takes
/// `&self` and can block.
pub trait ChallengeHandler: Send + Sync {
    /// Returns the answer, or `None` to abandon the attempt.
    fn answer(&self, challenge: AuthChallenge) -> Option<String>;
}

/// Set while any session exists, so the singleton rule is enforced.
static SESSION_TAKEN: AtomicBool = AtomicBool::new(false);

/// Guard object that releases the singleton on drop.
struct SessionGuard;

impl Drop for SessionGuard {
    fn drop(&mut self) {
        // A finished or failed attempt frees the slot so the user can retry.
        // The C library's own lock is released with `drm_lib_abort_init`.
        SESSION_TAKEN.store(false, Ordering::Release);
    }
}

/// A live `FairPlay` session.
///
/// Holding one means the bridge is initialised and, after [`Session::sign_in`]
/// returns, that keys can be requested.
pub struct Session {
    /// Released on drop so a retry is possible.
    _guard: SessionGuard,
    /// The state the bridge last reported.
    state: Mutex<SessionState>,
    /// Kept alive for the C callbacks: the C side holds a raw pointer to it.
    handler: Box<dyn ChallengeHandler>,
    /// Whether the C library's init was actually entered.
    ///
    /// `drm_lib_abort_init` is only meaningful after a call that failed to
    /// return, and calling it before any init runs drives the C library into an
    /// error path that prints to stderr and can disturb global state. Dropping a
    /// session that never signed in is an ordinary thing — a user opens the
    /// sign-in sheet and closes it — so the distinction is tracked rather than
    /// assumed.
    init_entered: AtomicBool,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").field("state", &*self.state.lock()).finish_non_exhaustive()
    }
}

impl Session {
    /// Claims the process-wide slot and builds a session.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::AlreadyInitialised`] when one already exists.
    /// A caller that wants to retry after a failure drops the previous session
    /// first, which releases the slot.
    pub fn new(handler: Box<dyn ChallengeHandler>) -> Result<Self, SignInError> {
        // `compare_exchange` rather than a load-then-store: two threads racing to
        // start a session is exactly the case the C library cannot survive.
        if SESSION_TAKEN.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err()
        {
            return Err(SignInError::AlreadyInitialised);
        }

        Ok(Self {
            _guard: SessionGuard,
            state: Mutex::new(SessionState::Starting),
            handler,
            init_entered: AtomicBool::new(false),
        })
    }

    /// The state the bridge last reported.
    #[must_use]
    pub fn state(&self) -> SessionState {
        self.state.lock().clone()
    }

    /// Loads Apple's libraries through the embedded `libhybris` core.
    ///
    /// This is separate from [`Session::sign_in`] so the two failure modes stay
    /// distinguishable: a missing payload is a setup problem, while a rejected
    /// password is an account problem, and the remedies are entirely different.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::RuntimeMissing`] when the payload is absent or
    /// incomplete, and [`SignInError::Bridge`] when it is present but will not
    /// load.
    pub fn load_runtime(&self, layout: &RuntimeLayout) -> Result<(), SignInError> {
        if !layout.verify().is_ready() {
            return Err(SignInError::RuntimeMissing);
        }

        let lib64 = cstring(layout.lib64.as_os_str())?;
        // The core is embedded and registered by name, not read from this path.
        let core_name = CString::new(RuntimeLayout::EMBEDDED_CORE_NAME)
            .map_err(|_| SignInError::InteriorNul)?;
        // libhybris itself is embedded too, so the linker directory is only a
        // label; the loader never reads from it.
        let linker_label = core_name.clone();

        // SAFETY: both strings outlive the call, and the C function either
        // retains nothing or copies what it needs before returning. It is
        // documented as safe to call once, which the session guard enforces.
        let rc = unsafe {
            ffi::hybris_backend_init(linker_label.as_ptr(), lib64.as_ptr(), core_name.as_ptr())
        };

        if rc == 0 {
            tracing::info!(target: "sosumi::fairplay", "Apple's libraries loaded in-process");
            Ok(())
        } else {
            // The C library writes its reason to stderr, so it is captured there
            // rather than returned. The message here names the step that failed.
            Err(SignInError::Bridge(format!(
                "libhybris could not load Apple's libraries (rc={rc})"
            )))
        }
    }

    /// Signs in, blocking until the bridge reports a result.
    ///
    /// `username` and `password` are `None` to reuse a stored session. When they
    /// are present, the bridge calls back for a verification code, which is
    /// answered through the [`ChallengeHandler`] given to [`Session::new`].
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Unanswered`] when the handler declines a challenge,
    /// and [`SignInError::Bridge`] with Apple's own wording when the attempt is
    /// rejected.
    pub fn sign_in(
        &self,
        layout: &RuntimeLayout,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<(), SignInError> {
        let base_dir = cstring(layout.session.as_os_str())?;
        let lib64 = cstring(layout.lib64.as_os_str())?;
        let username = username.map(cstring).transpose()?;
        let password = password.map(cstring).transpose()?;

        // The callback receives a pointer to the handler, which lives as long as
        // this session, so the pointer stays valid for the whole call.
        // Passing `self` to the C callbacks as an opaque handle. The cast to
        // `*mut` is required by the C signature; nothing on the C side writes
        // through it, and the validator reborrows it as `&Self`.
        #[allow(clippy::ref_as_ptr, clippy::ptr_as_ptr)]
        let handler_ptr = self as *const Self as *mut c_void;

        let config = ffi::DrmLibConfig {
            base_dir: base_dir.as_ptr(),
            lib64_dir: lib64.as_ptr(),
            username: username.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            password: password.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            // The C side derives a stable device identity when this is null.
            device_info: std::ptr::null(),
            // Zero selects the play method rather than forcing the download one.
            offline_only: 0,
            auth_cb: Some(auth_trampoline),
            auth_ud: handler_ptr,
            state_cb: Some(state_trampoline),
            state_ud: handler_ptr,
        };

        // Set before the call, not after: a call that hangs or faults has still
        // entered the C library, and the lock it took must be released.
        self.init_entered.store(true, Ordering::Release);

        // SAFETY: `config` points at strings that outlive the call, and at a
        // handler that lives as long as this session. `drm_lib_init` blocks until
        // the bridge has a result, so no callback fires after it returns.
        // Taking the address of a local for the duration of a call that does not
        // outlive it.
        #[allow(clippy::borrow_as_ptr)]
        let rc = unsafe { ffi::drm_lib_init(&config) };

        if rc == 0 {
            *self.state.lock() = SessionState::Running;
            tracing::info!(target: "sosumi::fairplay", "signed in; `FairPlay` lease acquired");
            Ok(())
        } else {
            *self.state.lock() = SessionState::Failed;
            // Apple's own wording, which is far more useful than a return code.
            let reason = self.last_error();
            if reason.is_empty() {
                Err(SignInError::Bridge(format!("sign-in failed (rc={rc})")))
            } else {
                Err(SignInError::Bridge(reason))
            }
        }
    }

    /// Why the last attempt failed, in Apple's words.
    ///
    /// Returns an empty string when the bridge has nothing to say, which is the
    /// case before any attempt has been made.
    #[must_use]
    pub fn last_error(&self) -> String {
        // SAFETY: the C function returns a pointer to a static buffer that it
        // documents as valid until the next attempt. It is never null.
        let raw = unsafe { ffi::hybris_last_error() };
        if raw.is_null() {
            return String::new();
        }
        // SAFETY: the pointer is non-null, NUL-terminated, and valid for the
        // duration of this borrow.
        unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned()
    }

    /// Releases the bridge's init lock after an attempt that did not complete.
    ///
    /// A no-op when init was never entered, which is the common case: a user who
    /// opens the sign-in sheet and closes it again never reaches the C library,
    /// and calling into it anyway would print an error and disturb global state
    /// for no reason.
    pub fn abort_init(&self) {
        if !self.init_entered.load(Ordering::Acquire) {
            return;
        }
        // SAFETY: documented as safe to call after `drm_lib_init` has returned,
        // whether it succeeded or failed. Only reached once init was entered.
        unsafe { ffi::drm_lib_abort_init() };
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Without this, a failed sign-in leaves the C library's init lock held,
        // and every later attempt hangs rather than reporting anything. The
        // symptom would look like a frozen application, so it is released here.
        self.abort_init();
    }
}

/// Turns a path into a C string for the boundary.
fn cstring(value: impl AsRef<std::ffi::OsStr>) -> Result<CString, SignInError> {
    use std::os::unix::ffi::OsStrExt as _;
    CString::new(value.as_ref().as_bytes()).map_err(|_| SignInError::InteriorNul)
}

/// The C auth callback, forwarded to the handler.
///
/// # Safety
///
/// Called by C with a valid `kind` pointer, a writable buffer of `out_size`
/// bytes, and the `auth_ud` that [`Session::sign_in`] passed, which is a pointer
/// to the live [`Session`].
unsafe extern "C" fn auth_trampoline(
    kind: *const c_char,
    out_buf: *mut c_char,
    out_size: c_int,
    ud: *mut c_void,
) {
    if kind.is_null() || out_buf.is_null() || ud.is_null() || out_size <= 1 {
        return;
    }

    // SAFETY: non-null and NUL-terminated, per the C contract above.
    let kind = unsafe { CStr::from_ptr(kind) }.to_string_lossy();
    let challenge = if kind == AuthChallenge::Credentials.as_c_str() {
        AuthChallenge::Credentials
    } else if kind == AuthChallenge::TwoFactor.as_c_str() {
        AuthChallenge::TwoFactor
    } else {
        tracing::warn!(target: "sosumi::fairplay", kind = %kind, "unknown auth challenge");
        return;
    };

    // SAFETY: `ud` is the pointer `sign_in` passed, and the session outlives the
    // call because `drm_lib_init` blocks until it finishes.
    let session = unsafe { &*(ud as *const Session) };

    let Some(answer) = session.handler.answer(challenge) else {
        tracing::warn!(target: "sosumi::fairplay", ?challenge, "sign-in challenge declined");
        return;
    };

    // Copy, leaving room for the terminator. A truncated answer is a wrong
    // answer, so it is logged rather than silently cut short.
    let bytes = answer.as_bytes();
    let capacity = (out_size - 1) as usize;
    if bytes.len() > capacity {
        tracing::warn!(
            target: "sosumi::fairplay",
            supplied = bytes.len(),
            capacity,
            "the answer does not fit the buffer the bridge provided"
        );
        return;
    }

    // SAFETY: the caller guarantees `out_buf` is writable for `out_size` bytes,
    // and the length was just checked against that.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), out_buf, bytes.len());
        *out_buf.add(bytes.len()) = 0;
    }
}

/// The C state callback, recorded on the session.
///
/// # Safety
///
/// Called by C with a valid `state` pointer and the `state_ud` from
/// [`Session::sign_in`], which is a pointer to the live [`Session`].
unsafe extern "C" fn state_trampoline(state: *const c_char, ud: *mut c_void) {
    if state.is_null() || ud.is_null() {
        return;
    }

    // SAFETY: non-null and NUL-terminated, per the C contract above.
    let name = unsafe { CStr::from_ptr(state) }.to_string_lossy();
    // SAFETY: `ud` is the pointer `sign_in` passed, valid for the call.
    let session = unsafe { &*(ud as *const Session) };

    let parsed = SessionState::from_c_str(&name);
    tracing::debug!(target: "sosumi::fairplay", state = %name, "bridge state");
    *session.state.lock() = parsed;
}

#[cfg(test)]
// Tests may panic; the workspace lint policy allows it under `cfg(test)` by the
// same convention every other crate in this workspace uses.
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeManifest;

    /// The singleton is process-wide, so tests that take one must not run
    /// concurrently. Sharing this lock is how they are serialised: the "second
    /// session is refused" test would otherwise pass or fail depending on which
    /// other test happened to hold the slot.
    static SESSION: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct Rejecting;
    impl ChallengeHandler for Rejecting {
        fn answer(&self, _: AuthChallenge) -> Option<String> {
            None
        }
    }

    struct Answering;
    impl ChallengeHandler for Answering {
        fn answer(&self, _: AuthChallenge) -> Option<String> {
            Some("123456".to_owned())
        }
    }

    fn empty_layout() -> (tempfile::TempDir, RuntimeLayout) {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = RuntimeLayout::under(
            dir.path(),
            RuntimeManifest { tier: "test".to_owned(), files: Vec::new() },
        );
        layout.ensure_directories().expect("directories");
        (dir, layout)
    }

    #[test]
    fn the_singleton_rule_is_enforced() {
        let _serialised = SESSION.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

        let first = Session::new(Box::new(Rejecting)).expect("first session");
        let second = Session::new(Box::new(Rejecting));
        assert!(matches!(second, Err(SignInError::AlreadyInitialised)));
        drop(first);
    }

    #[test]
    fn dropping_a_session_allows_a_retry() {
        let _serialised = SESSION.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

        // This is the whole point of the guard: a failed sign-in must not force
        // the user to restart the application.
        let first = Session::new(Box::new(Rejecting)).expect("first session");
        drop(first);
        let second = Session::new(Box::new(Rejecting));
        assert!(second.is_ok(), "a retry must be possible after a failure");
    }

    #[test]
    fn a_session_starts_in_the_starting_state() {
        let _serialised = SESSION.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

        let session = Session::new(Box::new(Rejecting)).expect("session");
        assert_eq!(session.state(), SessionState::Starting);
    }

    #[test]
    fn a_missing_payload_is_reported_before_any_library_call() {
        let _serialised = SESSION.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

        let (_dir, layout) = empty_layout();
        let session = Session::new(Box::new(Rejecting)).expect("session");
        // No files in the manifest means `verify` reports NotInstalled, so the
        // load must fail without touching the C library.
        assert!(matches!(session.load_runtime(&layout), Err(SignInError::RuntimeMissing)));
    }

    #[test]
    fn a_nul_byte_in_a_path_is_refused_rather_than_truncated() {
        let bad = "path\0with-nul";
        assert!(matches!(cstring(bad), Err(SignInError::InteriorNul)));
    }

    #[test]
    fn known_states_parse_and_unknown_ones_are_preserved() {
        assert_eq!(SessionState::from_c_str("RUNNING"), SessionState::Running);
        assert_eq!(SessionState::from_c_str("WAITING_2FA"), SessionState::WaitingForCode);
        assert_eq!(SessionState::from_c_str("LOGIN"), SessionState::LoggingIn);
        assert_eq!(SessionState::from_c_str("STARTING"), SessionState::Starting);
        assert_eq!(SessionState::from_c_str("INITIALIZING_FAIRPLAY"), SessionState::Preparing);
        assert_eq!(SessionState::from_c_str("FAILED"), SessionState::Failed);
        // A state added by a future C library must not be mistaken for a known
        // one, or the UI would claim something untrue.
        assert_eq!(
            SessionState::from_c_str("SOMETHING_NEW"),
            SessionState::Unknown("SOMETHING_NEW".to_owned())
        );
    }

    #[test]
    fn challenge_names_match_the_c_contract() {
        assert_eq!(AuthChallenge::Credentials.as_c_str(), "credentials");
        assert_eq!(AuthChallenge::TwoFactor.as_c_str(), "2fa");
    }

    #[test]
    fn a_handler_that_answers_is_used() {
        let _serialised = SESSION.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

        // Exercises the trampoline's happy path without a live C library.
        let session = Session::new(Box::new(Answering)).expect("session");
        assert_eq!(session.handler.answer(AuthChallenge::TwoFactor).as_deref(), Some("123456"));
    }
}
