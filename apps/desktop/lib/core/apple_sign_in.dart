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
/// Distinct from [SignInCancelled] and from a transient error: it means the
/// environment lacks something Apple's page requires.
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
/// # What actually decides success
///
/// Not the page's own reporting. Apple's account server sets the
/// `media-user-token` cookie once the credentials are accepted, and that can
/// happen before — or despite — anything the player reports afterwards. So this
/// watches the cookie store rather than the page's text or its navigation.
///
/// That matters here, because the page can show **"Authorization failed. Please
/// log in to enjoy Apple Music."** while the session has in fact been
/// established. That message comes from MusicKit failing to initialize, not from
/// the account server rejecting the credentials, and treating it as failure would
/// discard a perfectly good session.
///
/// # Why the engine sometimes cannot finish
///
/// MusicKit registers a DRM key system while starting up, by asking the engine
/// for `com.widevine.alpha`, `com.apple.fps`, or `com.microsoft.playready`
/// through `navigator.requestMediaKeySystemAccess`. WebKitGTK as distributions
/// build it has Encrypted Media compiled out, so that function does not exist and
/// MusicKit cannot initialize. See `docs/PLATFORM-LIMITS.md`.
///
/// The API is probed and reported, but a missing API does not by itself end the
/// attempt: the cookie is still awaited, because it is the only thing that
/// actually proves a session exists.
abstract final class AppleSignIn {
  /// How often to check the cookie store while the user signs in.
  static const _pollInterval = Duration(milliseconds: 700);

  /// How long to keep watching before concluding it will not complete.
  static const _sessionTimeout = Duration(minutes: 5);

  /// How long to give the page before probing its engine capabilities.
  static const _probeDelay = Duration(milliseconds: 2500);

  /// Reports whether the engine advertises Encrypted Media.
  ///
  /// Returns `null` when it does, or a short reason when it does not. A probe
  /// that cannot run returns a reason, because an unanswered question is not a
  /// pass.
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
    var closed = false;

    void finish(SignInResult result) {
      if (completed) return;
      completed = true;
      if (!completer.isCompleted) completer.complete(result);
    }

    // Probe the engine in the background. The result only shapes the message if
    // nothing else works out; it never ends the attempt on its own.
    final probe = Completer<String?>();
    unawaited(() async {
      await Future<void>.delayed(_probeDelay);
      probe.complete(await _probeEncryptedMedia(webview));
    }());

    // Watch the cookie store. This is the only signal that proves a session.
    unawaited(() async {
      final deadline = DateTime.now().add(_sessionTimeout);
      while (!completed && !closed && DateTime.now().isBefore(deadline)) {
        await Future<void>.delayed(_pollInterval);
        if (completed || closed) return;
        final token = await _readCookie(webview, 'media-user-token');
        if (token != null && token.isNotEmpty) {
          final storefront = await _readCookie(webview, 'geo') ?? 'us';
          finish(SignInSuccess(token, storefront.toLowerCase()));
          await webview.close();
          return;
        }
      }

      // The deadline passed. If the engine also lacks Encrypted Media, that is
      // the explanation; otherwise the user simply did not finish.
      final missing = await probe.future;
      if (missing != null) {
        finish(SignInUnavailable(
          'Signing in needs a browser engine with protected-media support, which '
          'this system\u2019s web view does not have.',
          'navigator.requestMediaKeySystemAccess reported: $missing',
        ));
        await webview.close();
      } else {
        finish(const SignInCancelled());
        await webview.close();
      }
    }());

    await webview.setOnUrlRequestCallback((url) => true);

    unawaited(webview.onClose.then((_) {
      closed = true;
      finish(const SignInCancelled());
    }));

    try {
      await webview.launch('https://music.apple.com/us/home');
    } on Object catch (error) {
      finish(SignInFailed('Could not load the sign-in page: $error'));
    }

    return completer.future;
  }

  /// Evaluates the capability script in the page.
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
        if (cookie.name == name && cookie.value.isNotEmpty) return cookie.value;
      }
    } on Object {
      // The engine may not have a cookie store yet; a null result lets the
      // caller retry on the next poll rather than surfacing an error.
    }
    return null;
  }
}
