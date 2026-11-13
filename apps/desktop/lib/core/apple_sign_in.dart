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

/// The sign-in window could not be opened, or the session could not be read.
final class SignInFailed extends SignInResult {
  const SignInFailed(this.message);

  final String message;
}

/// Apple ID sign-in, hosted in a web view.
///
/// This hosts **Apple's own** sign-in page rather than reimplementing account
/// authentication. That is the only correct approach, for three reasons in order
/// of importance:
///
/// 1. **Security.** The user types their Apple ID password into Apple's page,
///    over Apple's TLS, in a page Apple controls. Sosumi never sees it, never
///    stores it, and never handles it. A reimplemented form would mean this code
///    touching a password, which it should never do.
/// 2. **Two-factor authentication.** Apple's flow prompts for a verification
///    code on the user's trusted devices and handles the retry, the expiry, and
///    the trusted-device choice. Reproducing that state machine is a large
///    surface, and getting it wrong locks people out of their own accounts.
/// 3. **Correctness under change.** When Apple changes its sign-in, their page
///    changes and this keeps working. A reimplemented flow breaks silently.
///
/// What Sosumi reads afterwards is the `media-user-token` cookie, which is the
/// subscriber token the API expects. That is not a credential being extracted:
/// it identifies a session the user just authorised in Apple's own interface,
/// exactly as a browser would carry it forward. The password itself never
/// reaches this code.
///
/// Verified before building on it: WebKitGTK loads `music.apple.com`, renders the
/// web player, and sets the `media-user-token` cookie.
abstract final class AppleSignIn {
  /// The host the sign-in page and the web player share.
  static const _host = 'music.apple.com';

  /// How long to wait for the session cookie after the redirect lands.
  static const _cookieSettleDelay = Duration(milliseconds: 1500);

  /// Opens Apple's sign-in page and waits for the session cookie.
  ///
  /// Never throws: every outcome is a [SignInResult], because a closed window is
  /// a cancellation and a missing runtime is a message, not a crash.
  static Future<SignInResult> run() async {
    final Webview webview;
    try {
      if (!await WebviewWindow.isWebviewAvailable()) {
        return const SignInFailed('No web view runtime is available on this system.');
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

    void finish(SignInResult result) {
      if (completed) return;
      completed = true;
      if (!completer.isCompleted) completer.complete(result);
    }

    // Sign-in ends when navigation reaches the web player, which only happens
    // once Apple has established the session.
    await webview.setOnUrlRequestCallback((url) {
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
