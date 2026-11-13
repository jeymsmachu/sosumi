import 'dart:async';
import 'dart:ui' show Brightness;

import 'package:desktop_webview_window/desktop_webview_window.dart';

/// The result of a sign-in attempt.
sealed class SignInResult {
  const SignInResult();
}

/// The user authorised an account, and the session token was read.
final class SignInSuccess extends SignInResult {
  const SignInSuccess(this.userToken, this.storefront);

  /// The subscriber token, read from Apple's own session cookie.
  final String userToken;

  /// The storefront the account belongs to.
  final String storefront;
}

/// The user closed the sign-in window. Not a failure.
final class SignInCancelled extends SignInResult {
  const SignInCancelled();
}

/// Sign-in cannot complete on this machine, with a reason worth showing.
///
/// This is distinct from [SignInCancelled] and from a transient error: it means
/// the environment lacks something Apple's page requires, and no amount of
/// retrying will help.
final class SignInUnavailable extends SignInResult {
  const SignInUnavailable(this.reason, this.detail);

  /// One short line for the interface.
  final String reason;

  /// What exactly is missing, for the log and for a curious user.
  final String detail;
}

/// The sign-in window could not be opened or driven.
final class SignInFailed extends SignInResult {
  const SignInFailed(this.message);

  final String message;
}

/// Apple ID sign-in, hosted in Apple's own page.
///
/// # Why Apple's page and not a form of our own
///
/// The user types their Apple ID password into Apple's page, over Apple's TLS,
/// in a page Apple controls. Sosumi never sees it, never stores it, and never
/// handles it. Apple's two-factor step happens on Apple's page too, so there is
/// no state machine here to reproduce and nothing to get wrong. When Apple
/// changes its sign-in, their page changes and this keeps working.
///
/// # Why the engine has to support Encrypted Media
///
/// Apple's page is not a plain form. It loads MusicKit on the Web, which
/// registers a DRM key system as part of starting up. MusicKit asks the engine
/// for `com.widevine.alpha`, `com.apple.fps`, or `com.microsoft.playready` via
/// `navigator.requestMediaKeySystemAccess`. If the engine has no Encrypted Media
/// support, that call does not exist, MusicKit's initialization fails, and Apple
/// shows **"Authorization failed. Please log in to enjoy Apple Music."** — an
/// error about MusicKit, not about the password or the verification code the
/// user just entered correctly.
///
/// That is exactly the failure this code now detects *before* the user types
/// anything, so nobody is asked to enter a password into a page that cannot
/// finish.
///
/// WebKitGTK as distributions build it has Encrypted Media disabled at compile
/// time, so this is the common case on Linux rather than an edge case. See
/// `docs/PLATFORM-LIMITS.md`.
abstract final class AppleSignIn {
  /// The host the sign-in page and the web player share.
  static const _host = 'music.apple.com';

  /// How long to wait for the session cookie after the redirect lands.
  static const _cookieSettleDelay = Duration(milliseconds: 1500);

  /// How long to give the page before probing its capabilities.
  static const _probeDelay = Duration(milliseconds: 1200);

  /// Asks the engine whether it can resolve a DRM key system.
  ///
  /// Returns `null` when the engine advertises Encrypted Media, or a
  /// human-readable reason when it does not.
  static const _probeScript = '''
    (function () {
      if (typeof navigator.requestMediaKeySystemAccess !== 'function') {
        return 'no-api';
      }
      if (typeof window.MediaKeys !== 'function') {
        return 'no-mediakeys';
      }
      return null;
    })()
  ''';

  /// Opens Apple's sign-in page and waits for the session cookie.
  ///
  /// Never throws: every outcome is a [SignInResult], because a closed window is
  /// a cancellation and a missing engine capability is a message, not a crash.
  static Future<SignInResult> run() async {
    final Webview webview;
    try {
      if (!await WebviewWindow.isWebviewAvailable()) {
        return const SignInUnavailable(
          'No web view is available on this system.',
          'The desktop web view runtime could not be found.',
        );
      }
      webview = await WebviewWindow.create(
        // A sheet-sized window, so sign-in reads as part of the app rather than
        // as launching a separate browser.
        configuration: const CreateConfiguration(
          title: 'Sign in to Apple Music',
          windowWidth: 460,
          windowHeight: 640,
          brightness: Brightness.dark,
          // Hide the browser chrome. This is a focused task, not a browsing
          // session, and a URL bar would invite the user to navigate away.
          titleBar: TitleBarOptions(
            showNavigation: false,
            showReload: false,
            showTitle: true,
            showHost: false,
          ),
        ),
      );
    } on Object catch (error) {
      return SignInFailed('Could not open the sign-in window: $error');
    }

    final completer = Completer<SignInResult>();
    var completed = false;
    var probed = false;

    void finish(SignInResult result) {
      if (completed) return;
      completed = true;
      if (!completer.isCompleted) completer.complete(result);
    }

    await webview.setOnUrlRequestCallback((url) {
      // Probe once, as soon as Apple's page is up. Doing it before the user
      // types means nobody enters a password into a form that cannot complete.
      if (!probed && url.contains(_host)) {
        probed = true;
        unawaited(() async {
          await Future<void>.delayed(_probeDelay);
          final missing = await _probeEncryptedMedia(webview);
          if (missing != null) {
            finish(SignInUnavailable(
              'Signing in needs a browser engine with protected-media support, '
              'which this system\u2019s web view does not have.',
              'navigator.requestMediaKeySystemAccess reported: $missing',
            ));
            await webview.close();
          }
        }());
      }

      // Sign-in ends when navigation reaches the web player, which only happens
      // once Apple has established the session.
      if (url.contains(_host) && !url.contains('/signin') && !url.contains('/login')) {
        unawaited(() async {
          await Future<void>.delayed(_cookieSettleDelay);
          final token = await _readCookie(webview, 'media-user-token');
          if (token != null && token.isNotEmpty) {
            final storefront = await _readCookie(webview, 'geo') ?? 'us';
            finish(SignInSuccess(token, storefront.toLowerCase()));
            await webview.close();
          }
        }());
      }
      // Never block navigation: the page has to load.
      return true;
    });

    // Closing the window without authorising is a cancellation.
    unawaited(webview.onClose.then((_) => finish(const SignInCancelled())));

    try {
      await webview.launch('https://music.apple.com/us/home');
    } on Object catch (error) {
      finish(SignInFailed('Could not load the sign-in page: $error'));
    }

    return completer.future;
  }

  /// Evaluates the capability script in the page.
  ///
  /// Returns `null` when the engine supports Encrypted Media, or the reported
  /// reason when it does not, or when the probe itself could not run — a probe
  /// that cannot answer must not be treated as a pass.
  static Future<String?> _probeEncryptedMedia(Webview webview) async {
    try {
      final result = await webview.evaluateJavaScript(_probeScript);
      final value = result?.toString();
      if (value == null || value == 'null') return null;
      return value;
    } on Object catch (error) {
      return 'probe could not run ($error)';
    }
  }

  /// Reads one cookie by name from the web view's store.
  static Future<String?> _readCookie(Webview webview, String name) async {
    try {
      final cookies = await webview.getAllCookies();
      for (final cookie in cookies) {
        if (cookie.name == name) return cookie.value;
      }
    } on Object {
      // The engine may not have a cookie store yet; a null result lets the
      // caller retry on the next navigation rather than surfacing an error.
    }
    return null;
  }
}
