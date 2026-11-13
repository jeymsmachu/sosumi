import 'package:flutter/material.dart';

import '../../core/install.dart';
import '../../theme/theme.dart';

/// A determinate progress row for the lossless install.
///
/// One widget for both places the install can be started — the sign-in sheet and
/// Settings — so the two cannot disagree about what is happening.
///
/// It is determinate rather than a spinner because the work has a known shape:
/// a fixed list of files, each with a reported size. `progress-indicators.md`:
/// "When possible, use a determinate progress indicator" and "Be as accurate as
/// possible when reporting advancement". The bar counts files rather than bytes
/// across the whole run, because that is the figure the caller can compute
/// without waiting on 22 separate `Content-Length` headers.
class InstallProgress extends StatelessWidget {
  const InstallProgress({super.key, required this.state});

  final InstallState state;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    // Idle: nothing to say. The caller decides what to show instead.
    if (state.phase == InstallPhase.idle) return const SizedBox.shrink();

    final (label, detail) = _copy();
    final progress = state.overallProgress;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                label,
                style: TextStyles.headline.copyWith(color: palette.textPrimary),
              ),
            ),
            if (state.files > 0 && state.phase != InstallPhase.done)
              Text(
                '${state.index} of ${state.files}',
                style: TextStyles.numeric.copyWith(color: palette.textSecondary),
              ),
          ],
        ),
        const SizedBox(height: Insets.sm),
        // The bar animates its value rather than jumping, so a fast file still
        // reads as progress instead of a flicker. `progress-indicators.md` asks
        // for the pace to be evened out.
        TweenAnimationBuilder(
          tween: Tween(begin: 0.0, end: progress),
          duration: context.motionDuration(Motion.normal),
          curve: context.motionCurve(Motion.enter),
          builder: (context, value, _) => ClipRRect(
            borderRadius: BorderRadius.circular(Radii.pill),
            child: LinearProgressIndicator(
              value: value,
              minHeight: 4,
              backgroundColor: palette.fill,
              valueColor: AlwaysStoppedAnimation(
                state.phase == InstallPhase.failed ? palette.accent : palette.blue,
              ),
            ),
          ),
        ),
        if (detail != null) ...[
          const SizedBox(height: Insets.sm),
          Text(detail, style: TextStyles.caption1.copyWith(color: palette.textSecondary)),
        ],
      ],
    );
  }

  /// The heading and the line beneath it, for the current phase.
  ///
  /// Kept as one switch so the two lines cannot describe different things.
  (String, String?) _copy() {
    switch (state.phase) {
      case InstallPhase.idle:
        return ('', null);
      case InstallPhase.running:
        final file = state.file;
        final bytes = _bytes();
        // A file name is more useful than the word "downloading", and `writing.md`
        // asks for specifics over filler. The byte count is there because a name
        // alone says nothing about how long it will take.
        return (
          'Downloading',
          file == null ? bytes : '$file${bytes.isEmpty ? '' : ' · $bytes'}',
        );
      case InstallPhase.done:
        return (
          'Installed',
          state.fetched == 0
              ? 'Everything was already in place.'
              : '${state.fetched} file${state.fetched == 1 ? '' : 's'} downloaded and verified.',
        );
      case InstallPhase.cancelled:
        // Says what was kept, because that is the thing a person wants to know
        // after stopping a long download.
        return ('Stopped', 'Finished files were kept. Install again to resume.');
      case InstallPhase.failed:
        return ('Download failed', state.error);
    }
  }

  /// A transfer reading for the current file, when one is known.
  String _bytes() {
    final total = state.fileTotal;
    final received = state.received;
    if (total == null) return received > 0 ? '${_size(received)} of unknown size' : '';
    return '${_size(received)} of ${_size(total)}';
  }

  /// A byte count in the units a person reads.
  static String _size(int bytes) {
    const units = ['B', 'KB', 'MB', 'GB'];
    var value = bytes.toDouble();
    var unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit++;
    }
    return '${value.toStringAsFixed(value >= 100 || unit == 0 ? 0 : 1)} ${units[unit]}';
  }
}
