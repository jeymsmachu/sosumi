import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/player.dart';
import '../../theme/theme.dart';

/// The floating player.
///
/// Apple moved the transport to a floating control at the bottom of the window,
/// so the glass reads over the content behind it. This mirrors that: a rounded
/// panel, inset from the edges, carrying artwork, title, transport, and a
/// scrubbable progress line.
class PlayerBar extends ConsumerWidget {
  const PlayerBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final player = ref.watch(playerProvider);
    final controller = ref.read(playerProvider.notifier);

    return Padding(
      padding: const EdgeInsets.fromLTRB(Insets.md, 0, Insets.md, Insets.md),
      child: GlassSurface(
        radius: Radii.player,
        blur: 32,
        padding: const EdgeInsets.symmetric(
          horizontal: Insets.md,
          vertical: Insets.sm,
        ),
        child: Row(
          children: [
            _Artwork(player: player),
            const SizedBox(width: Insets.md),
            Expanded(
              flex: 3,
              child: _TrackInfo(player: player),
            ),
            _Transport(
              player: player,
              onPlayPause: controller.togglePlayPause,
              onNext: controller.next,
              onPrevious: () {},
            ),
            const SizedBox(width: Insets.md),
            Expanded(
              flex: 3,
              child: _Progress(player: player),
            ),
            const SizedBox(width: Insets.md),
            _Volume(),
          ],
        ),
      ),
    );
  }
}

class _Artwork extends StatelessWidget {
  const _Artwork({required this.player});

  final PlayerState player;

  @override
  Widget build(BuildContext context) {
    final url = player.track?.artworkUrl;
    return ClipRRect(
      borderRadius: BorderRadius.circular(Radii.small),
      child: SizedBox(
        width: 48,
        height: 48,
        child: url == null
            ? ColoredBox(
                color: context.palette.glassFill,
                child: Icon(
                  Icons.music_note_rounded,
                  size: 20,
                  color: context.palette.textSecondary,
                ),
              )
            : Image.network(
                url,
                fit: BoxFit.cover,
                errorBuilder: (_, _, _) => ColoredBox(
                  color: context.palette.glassFill,
                  child: Icon(
                    Icons.music_note_rounded,
                    size: 20,
                    color: context.palette.textSecondary,
                  ),
                ),
              ),
      ),
    );
  }
}

class _TrackInfo extends StatelessWidget {
  const _TrackInfo({required this.player});

  final PlayerState player;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final track = player.track;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisAlignment: MainAxisAlignment.center,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          track?.title ?? 'Nothing playing',
          style: TextStyles.rowTitle.copyWith(color: palette.textPrimary),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 2),
        Text(
          track?.artist ?? 'Choose something to listen to',
          style: TextStyles.rowSubtitle.copyWith(color: palette.textSecondary),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
      ],
    );
  }
}

class _Transport extends StatelessWidget {
  const _Transport({
    required this.player,
    required this.onPlayPause,
    required this.onNext,
    required this.onPrevious,
  });

  final PlayerState player;
  final VoidCallback onPlayPause;
  final VoidCallback onNext;
  final VoidCallback onPrevious;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final playing = player.status == PlaybackStatus.playing;

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        _TransportButton(
          icon: Icons.skip_previous_rounded,
          tooltip: 'Previous',
          onTap: onPrevious,
        ),
        const SizedBox(width: Insets.xs),
        // The play control is larger and uses the primary colour, which is the
        // strongest affordance in the bar.
        InteractiveGlass(
          onTap: onPlayPause,
          radius: Radii.pill,
          padding: const EdgeInsets.all(Insets.sm),
          child: Icon(
            playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
            size: 26,
            color: palette.textPrimary,
          ),
        ),
        const SizedBox(width: Insets.xs),
        _TransportButton(
          icon: Icons.skip_next_rounded,
          tooltip: 'Next',
          onTap: onNext,
        ),
      ],
    );
  }
}

class _TransportButton extends StatelessWidget {
  const _TransportButton({
    required this.icon,
    required this.tooltip,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InteractiveGlass(
        onTap: onTap,
        radius: Radii.pill,
        padding: const EdgeInsets.all(Insets.sm),
        child: Icon(icon, size: 20, color: context.palette.textSecondary),
      ),
    );
  }
}

class _Progress extends StatelessWidget {
  const _Progress({required this.player});

  final PlayerState player;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final progress = player.progress ?? 0.0;

    return Row(
      children: [
        Text(
          _format(player.position),
          style: TextStyles.numeric.copyWith(color: palette.textSecondary),
        ),
        const SizedBox(width: Insets.sm),
        Expanded(
          child: ClipRRect(
            borderRadius: BorderRadius.circular(Radii.pill),
            child: LinearProgressIndicator(
              value: progress,
              minHeight: 4,
              backgroundColor: palette.textSecondary.withValues(alpha: 0.2),
              valueColor: AlwaysStoppedAnimation(palette.textPrimary),
            ),
          ),
        ),
        const SizedBox(width: Insets.sm),
        Text(
          player.duration == null ? '--:--' : _format(player.duration!),
          style: TextStyles.numeric.copyWith(color: palette.textSecondary),
        ),
      ],
    );
  }

  static String _format(Duration duration) {
    final minutes = duration.inMinutes;
    final seconds = duration.inSeconds.remainder(60).toString().padLeft(2, '0');
    return '$minutes:$seconds';
  }
}

class _Volume extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(Icons.volume_down_rounded, size: 16, color: palette.textSecondary),
        const SizedBox(width: Insets.sm),
        SizedBox(
          width: 88,
          child: ClipRRect(
            borderRadius: BorderRadius.circular(Radii.pill),
            child: LinearProgressIndicator(
              value: 0.7,
              minHeight: 4,
              backgroundColor: palette.textSecondary.withValues(alpha: 0.2),
              valueColor: AlwaysStoppedAnimation(palette.textSecondary),
            ),
          ),
        ),
      ],
    );
  }
}
