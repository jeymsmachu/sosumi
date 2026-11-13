import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../rust/api.dart';
import 'engine.dart';

/// The current session, mirrored from the Rust core.
///
/// The core is the source of truth: it holds the tokens and persists them. This
/// is a display projection, plus the two fields the sidebar needs to draw the
/// account row.
class SessionState {
  const SessionState({
    this.signedIn = false,
    this.playable = false,
    this.storefront = 'us',
    this.displayName,
    this.avatarUrl,
  });

  final bool signedIn;
  final bool playable;
  final String storefront;

  /// The account's display name, when the core knows it.
  final String? displayName;

  /// The account's avatar URL, when the core knows it.
  final String? avatarUrl;

  SessionState copyWith({
    bool? signedIn,
    bool? playable,
    String? storefront,
    String? displayName,
    String? avatarUrl,
  }) {
    return SessionState(
      signedIn: signedIn ?? this.signedIn,
      playable: playable ?? this.playable,
      storefront: storefront ?? this.storefront,
      displayName: displayName ?? this.displayName,
      avatarUrl: avatarUrl ?? this.avatarUrl,
    );
  }
}

/// Notifies listeners when the session changes.
class SessionController extends StateNotifier<SessionState> {
  SessionController(this._engine) : super(const SessionState()) {
    _refresh();
  }

  final Engine _engine;

  Future<void> _refresh() async {
    final dto = await _engine.session();
    state = SessionState(
      signedIn: dto.signedIn,
      playable: dto.playable,
      storefront: dto.storefront,
    );
  }

  /// Records a session obtained outside the app, then refreshes.
  ///
  /// The token is never logged and never leaves the core.
  Future<void> setSession({
    required String musicUserToken,
    required String storefront,
  }) async {
    final dto = await _engine.setSession(
      musicUserToken: musicUserToken,
      storefront: storefront,
    );
    state = SessionState(
      signedIn: dto.signedIn,
      playable: dto.playable,
      storefront: dto.storefront,
    );
  }

  /// Signs out and clears local session state.
  Future<void> signOut() async {
    await _engine.signOut();
    await _refresh();
  }

  /// Re-reads the session from the core.
  Future<void> refresh() => _refresh();
}

/// The session controller, derived from the engine.
final sessionProvider = StateNotifierProvider<SessionController, SessionState>((ref) {
  final engine = ref.watch(engineProvider).requireValue;
  return SessionController(engine);
});
