import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../rust/api.dart';
import 'engine.dart';

/// What the player is showing.
enum PlaybackStatus { idle, loading, playing, paused }

/// The player's display state.
class PlayerState {
  const PlayerState({
    this.status = PlaybackStatus.idle,
    this.track,
    this.position = Duration.zero,
    this.duration,
    this.queueLength = 0,
    this.error,
  });

  final PlaybackStatus status;
  final TrackDto? track;
  final Duration position;
  final Duration? duration;
  final int queueLength;

  /// The most recent error message, cleared once shown.
  final String? error;

  /// Progress in the range `0..1`, or `null` when the duration is unknown.
  double? get progress {
    final total = duration;
    if (total == null || total.inMilliseconds == 0) return null;
    return (position.inMilliseconds / total.inMilliseconds).clamp(0.0, 1.0);
  }

  PlayerState copyWith({
    PlaybackStatus? status,
    TrackDto? track,
    Duration? position,
    Duration? duration,
    int? queueLength,
    String? error,
    bool clearError = false,
    bool clearTrack = false,
  }) {
    return PlayerState(
      status: status ?? this.status,
      track: clearTrack ? null : (track ?? this.track),
      position: position ?? this.position,
      duration: duration ?? this.duration,
      queueLength: queueLength ?? this.queueLength,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Consumes the Rust event stream and exposes it as player state.
///
/// The core publishes events rather than the UI polling, so the player is idle
/// between changes. Events arrive on a Dart stream from the bridge; this class
/// translates them into the small amount of state the UI needs.
class PlayerController extends StateNotifier<PlayerState> {
  PlayerController(this._engine) : super(const PlayerState());

  final Engine _engine;
  StreamSubscription<EventDto>? _subscription;

  /// Begins listening to the core's event stream.
  void attach(Stream<EventDto> events) {
    _subscription?.cancel();
    _subscription = events.listen(_onEvent);
  }

  void _onEvent(EventDto event) {
    switch (event.kind) {
      case 'track_changed':
        state = state.copyWith(
          status: PlaybackStatus.playing,
          track: TrackDto(
            id: event.trackId ?? '',
            title: event.title ?? '',
            artist: event.artist ?? '',
            artworkUrl: event.artworkUrl,
            durationMs: event.durationMs,
          ),
          position: Duration.zero,
          duration: event.durationMs == null
              ? null
              : Duration(milliseconds: event.durationMs!),
          clearError: true,
        );
      case 'playing':
        state = state.copyWith(status: PlaybackStatus.playing);
      case 'paused':
        state = state.copyWith(status: PlaybackStatus.paused);
      case 'stopped':
        state = state.copyWith(status: PlaybackStatus.idle, clearTrack: true);
      case 'ended':
        state = state.copyWith(status: PlaybackStatus.idle);
      case 'position':
        state = state.copyWith(
          position: Duration(milliseconds: event.positionMs ?? 0),
        );
      case 'queue_changed':
        state = state.copyWith(queueLength: event.queueLength ?? 0);
      case 'error':
        state = state.copyWith(error: event.message);
      default:
        // Forward-compatible: an event kind this build does not know is
        // ignored rather than treated as an error.
        break;
    }
  }

  /// Plays a track, replacing what is loaded.
  Future<void> play(String trackId) async {
    state = state.copyWith(status: PlaybackStatus.loading, clearError: true);
    try {
      final track = await _engine.playTrack(id: trackId);
      state = state.copyWith(
        status: PlaybackStatus.playing,
        track: track,
        position: Duration.zero,
        duration: track.durationMs == null
            ? null
            : Duration(milliseconds: track.durationMs!),
      );
    } on Exception catch (error) {
      state = state.copyWith(status: PlaybackStatus.idle, error: '$error');
    }
  }

  /// Plays the next track in the queue.
  Future<void> next() async {
    try {
      final track = await _engine.next();
      if (track != null) {
        state = state.copyWith(
          status: PlaybackStatus.playing,
          track: track,
          position: Duration.zero,
        );
      }
    } on Exception catch (error) {
      state = state.copyWith(error: '$error');
    }
  }

  /// Clears the pending error once the UI has displayed it.
  void dismissError() {
    state = state.copyWith(clearError: true);
  }

  @override
  void dispose() {
    _subscription?.cancel();
    super.dispose();
  }
}

/// The player controller, derived from the engine.
final playerProvider = StateNotifierProvider<PlayerController, PlayerState>((ref) {
  final engine = ref.watch(engineProvider).requireValue;
  return PlayerController(engine);
});
