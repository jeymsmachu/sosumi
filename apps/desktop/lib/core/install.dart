import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../rust/api.dart';
import 'engine.dart';

/// Where an install is.
enum InstallPhase {
  /// Nothing is running and nothing is needed.
  idle,

  /// Fetching. The state carries the progress.
  running,

  /// Finished successfully.
  done,

  /// The user stopped it. Kept distinct from [InstallPhase.done] so the screen
  /// can say which happened.
  cancelled,

  /// It failed, with [InstallState.error] set.
  failed,
}

/// The install's state, in the shape the progress row renders.
class InstallState {
  const InstallState({
    this.phase = InstallPhase.idle,
    this.file,
    this.index = 0,
    this.files = 0,
    this.received = 0,
    this.fileTotal,
    this.error,
    this.fetched = 0,
  });

  final InstallPhase phase;

  /// The file being fetched.
  final String? file;

  /// One-based position of the current file.
  final int index;

  /// How many files are queued.
  final int files;

  /// Bytes received of the current file.
  final int received;

  /// The current file's size, when the server reported one.
  final int? fileTotal;

  /// A message fit for the screen, when the phase is [InstallPhase.failed].
  final String? error;

  /// How many files the run downloaded, once it finished.
  final int fetched;

  /// Whether an install is in flight.
  bool get isRunning => phase == InstallPhase.running;

  /// How far the current file has come, in `0..1`, or `null` when its size is
  /// not yet known.
  ///
  /// Per file rather than per run because the server reports lengths per file,
  /// and a bar that jumped at each boundary would be less honest than one that
  /// reads accurately for the file in front of it.
  double? get fileProgress {
    final total = fileTotal;
    if (total == null || total == 0) return null;
    return (received / total).clamp(0.0, 1.0);
  }

  /// How far the whole run has come, in `0..1`.
  ///
  /// Finished files plus the fraction of the current one, so the bar only ever
  /// advances.
  double get overallProgress {
    if (files == 0) return 0;
    final finished = (index - 1).clamp(0, files);
    return ((finished + (fileProgress ?? 0)) / files).clamp(0.0, 1.0);
  }

  InstallState copyWith({
    InstallPhase? phase,
    String? file,
    int? index,
    int? files,
    int? received,
    int? fileTotal,
    String? error,
    int? fetched,
    bool clearFile = false,
    bool clearError = false,
    bool clearFileTotal = false,
  }) {
    return InstallState(
      phase: phase ?? this.phase,
      file: clearFile ? null : (file ?? this.file),
      index: index ?? this.index,
      files: files ?? this.files,
      received: received ?? this.received,
      fileTotal: clearFileTotal ? null : (fileTotal ?? this.fileTotal),
      error: clearError ? null : (error ?? this.error),
      fetched: fetched ?? this.fetched,
    );
  }
}

/// Drives the lossless install and exposes its progress.
///
/// The subscription is opened before the work starts, which the engine's
/// long-lived channel makes safe in either order. That ordering is what stops an
/// install from reporting into a channel with no listeners — the failure mode a
/// spinner that never resolves comes from.
class InstallController extends StateNotifier<InstallState> {
  InstallController(this._engine) : super(const InstallState());

  final Engine _engine;
  StreamSubscription<InstallEventDto>? _events;

  /// Starts the install, or returns immediately when one is already running.
  Future<void> start() async {
    if (state.isRunning) return;

    // Any previous subscription is dropped first, so a retry after a failure
    // cannot double-handle events.
    await _events?.cancel();

    state = const InstallState(phase: InstallPhase.running, index: 1);

    _events = _engine.watchInstall().listen(_onEvent);

    try {
      await _engine.beginLosslessInstall();
    } on Exception catch (error) {
      state = state.copyWith(phase: InstallPhase.failed, error: _describe(error));
    }
  }

  /// Stops an install in progress. Files already verified are kept.
  Future<void> cancel() async {
    if (!state.isRunning) return;
    await _engine.cancelLosslessInstall();
  }

  void _onEvent(InstallEventDto event) {
    switch (event.kind) {
      case 'started':
        state = state.copyWith(
          phase: InstallPhase.running,
          files: event.files,
          index: 1,
          received: 0,
          clearFile: true,
          clearFileTotal: true,
          clearError: true,
        );
      case 'progress':
        state = state.copyWith(
          phase: InstallPhase.running,
          file: event.file,
          index: event.index,
          files: event.files,
          received: event.done.toInt(),
          fileTotal: event.total?.toInt(),
          clearFileTotal: event.total == null,
        );
      case 'verifying':
        // The bar holds at full for this file while the checksum is compared,
        // which is honest: the bytes are in, and the wait is real.
        state = state.copyWith(
          phase: InstallPhase.running,
          file: event.file,
          index: event.index,
          files: event.files,
          received: state.fileTotal ?? state.received,
        );
      case 'finished':
        state = state.copyWith(
          phase: InstallPhase.done,
          fetched: event.index,
          clearFile: true,
          clearFileTotal: true,
        );
      case 'cancelled':
        state = state.copyWith(phase: InstallPhase.cancelled, clearFile: true);
      case 'failed':
        state = state.copyWith(
          phase: InstallPhase.failed,
          error: event.message ?? 'The download failed.',
          clearFile: true,
        );
      default:
        // Forward-compatible: an unknown kind is ignored rather than treated as
        // a failure, so a newer bridge does not break an older UI.
        break;
    }
  }

  /// Clears a finished, cancelled, or failed state back to idle, so the row
  /// returns to its resting appearance.
  void reset() {
    if (state.isRunning) return;
    state = const InstallState();
  }

  /// Turns a bridge error into something worth reading.
  String _describe(Exception error) {
    final text = error.toString();
    if (text.contains('already in progress')) {
      return 'An install is already running.';
    }
    return text;
  }

  @override
  void dispose() {
    _events?.cancel();
    super.dispose();
  }
}

/// The install controller, derived from the engine.
final installProvider = StateNotifierProvider<InstallController, InstallState>((ref) {
  final engine = ref.watch(engineProvider).requireValue;
  return InstallController(engine);
});
