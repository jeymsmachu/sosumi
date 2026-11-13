import 'package:flutter_test/flutter_test.dart';
import 'package:sosumi/core/install.dart';

/// The progress arithmetic, which the UI trusts to draw an honest bar.
///
/// These are worth testing because a wrong figure here is invisible in review —
/// the bar still moves, it just lies — and because the whole-run fraction is the
/// one piece of UI logic that combines two different scales (files and bytes).
void main() {
  group('InstallState progress', () {
    test('a fresh state reports no progress rather than a full bar', () {
      const state = InstallState();
      expect(state.overallProgress, 0);
      expect(state.fileProgress, isNull);
      expect(state.isRunning, isFalse);
    });

    test('a file with no reported length has no per-file progress', () {
      // The honest answer when the server sent no Content-Length: a bar that
      // guessed would advance at a made-up rate.
      const state = InstallState(files: 4, index: 1, received: 500, fileTotal: null);
      expect(state.fileProgress, isNull);
    });

    test('per-file progress reflects the bytes of the current file', () {
      const state = InstallState(files: 4, index: 2, received: 250, fileTotal: 1000);
      expect(state.fileProgress, 0.25);
    });

    test('whole-run progress counts finished files plus the current fraction', () {
      // Two of four done, half of the third: (2 + 0.5) / 4.
      const state = InstallState(files: 4, index: 3, received: 500, fileTotal: 1000);
      expect(state.overallProgress, closeTo(0.625, 0.0001));
    });

    test('whole-run progress never exceeds one', () {
      // A server that over-reports, or a final chunk arriving twice, must not
      // push the bar past its end.
      const state = InstallState(files: 2, index: 2, received: 4000, fileTotal: 1000);
      expect(state.overallProgress, 1.0);
    });

    test('a zero-length file does not divide by zero', () {
      const state = InstallState(files: 1, index: 1, received: 0, fileTotal: 0);
      expect(state.fileProgress, isNull);
      expect(state.overallProgress, 0);
    });

    test('copyWith clears only what it is asked to', () {
      const state = InstallState(files: 3, index: 2, file: 'a.so', fileTotal: 10, error: 'x');
      final verifying = state.copyWith(clearFile: true, clearFileTotal: true, clearError: true);
      expect(verifying.file, isNull);
      expect(verifying.fileTotal, isNull);
      expect(verifying.error, isNull);
      // Untouched fields survive, which is what makes the event handling a set
      // of small nudges rather than a rebuild per event.
      expect(verifying.files, 3);
      expect(verifying.index, 2);
    });

    test('done outranks running, so a late event cannot restart the row', () {
      const done = InstallState(phase: InstallPhase.done);
      expect(done.isRunning, isFalse);
    });
  });
}
