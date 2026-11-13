import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/player.dart';
import '../../theme/theme.dart';

/// The floating transport.
///
/// Apple moved the transport into a floating capsule at the bottom of the
/// window rather than a strip docked to the edge, so the material reads over the
/// content scrolling beneath it. This is the same shape, and the same
/// proportions: a centred capsule roughly half the window's width, not a
/// full-bleed bar.
///
/// It is one of only two glass surfaces in the app, which is what keeps the
/// material meaning "this is a control" rather than "this is a panel".
class PlayerBar extends ConsumerWidget {
  const PlayerBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final player = ref.watch(playerProvider);
    final controller = ref.read(playerProvider.notifier);
    final palette = context.palette;

    return Padding(
      padding: const EdgeInsets.only(bottom: Sizes.playerInset),
      child: Center(
        child: ConstrainedBox(
          // A max width keeps the capsule from stretching across a wide monitor,
          // which is what made it read as a toolbar instead of a floating
          // control. It grows with the window up to this cap.
          constraints: const BoxConstraints(maxWidth: 880),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: Sizes.playerInset),
            child: GlassSurface(
              radius: Radii.player,
              blur: 34,
              fill: palette.playerFill,
              padding: const EdgeInsets.symmetric(horizontal: Insets.md),
              child: SizedBox(
                height: Sizes.playerHeight,
                child: Row(
                  children: [
                    _Artwork(player: player),
                    const SizedBox(width: Insets.md),
                    Flexible(
                      flex: 3,
                      child: _TrackInfo(player: player),
                    ),
                    const Spacer(),
                    _Transport(
                      player: player,
                      onPlayPause: controller.togglePlayPause,
                      onNext: controller.next,
                      onPrevious: () {},
                    ),
                    const Spacer(),
                    Flexible(
                      flex: 3,
                      child: _Progress(player: player),
                    ),
                    const SizedBox(width: Insets.sm),
                    const _Volume(),
                  ],
                ),
              ),
            ),
          ),
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
    final palette = context.palette;
    final url = player.track?.artworkUrl;

    return ClipRRect(
      borderRadius: BorderRadius.circular(Radii.small),
      child: SizedBox(
        width: 44,
        height: 44,
        child: url == null
            ? ColoredBox(color: palette.fill, child: _placeholder(palette))
            : Image.network(
                url,
                fit: BoxFit.cover,
                errorBuilder: (_, _, _) =>
                    ColoredBox(color: palette.fill, child: _placeholder(palette)),
              ),
      ),
    );
  }

  Widget _placeholder(SosumiPalette palette) =>
      Icon(Icons.music_note_rounded, size: 18, color: palette.textTertiary);
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
          track?.title ?? 'Nothing Playing',
          style: TextStyles.body.copyWith(color: palette.textPrimary),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 1),
        Text(
          track?.artist ?? 'Choose something to listen to',
          style: TextStyles.subheadline.copyWith(color: palette.textSecondary),
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
    final playing = player.status == PlaybackStatus.playing;
    final enabled = player.track != null;

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        _TransportButton(
          icon: Icons.skip_previous_rounded,
          tooltip: 'Previous',
          onTap: enabled ? onPrevious : null,
        ),
        // The play control is the primary action, so it alone gets a larger
        // target and the primary text colour.
        _TransportButton(
          icon: playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
          tooltip: playing ? 'Pause' : 'Play',
          size: 22,
          onTap: enabled ? onPlayPause : null,
        ),
        _TransportButton(
          icon: Icons.skip_next_rounded,
          tooltip: 'Next',
          onTap: enabled ? onNext : null,
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
    this.size = 17,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onTap;
  final double size;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return InteractiveGlass(
      onTap: onTap,
      radius: Radii.pill,
      tooltip: tooltip,
      padding: const EdgeInsets.all(Insets.sm),
      child: Icon(
        icon,
        size: size,
        color: onTap == null ? palette.textTertiary : palette.textPrimary,
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
    final progress = player.progress;
    final enabled = progress != null;

    return Row(
      children: [
        Text(
          _format(player.position),
          style: TextStyles.numeric.copyWith(color: palette.textSecondary),
        ),
        const SizedBox(width: Insets.sm),
        Expanded(
          child: Scrubber(
            value: progress ?? 0,
            activeColor: enabled ? palette.textSecondary : palette.textTertiary,
            onChanged: enabled ? (_) {} : null,
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
  const _Volume();

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(Icons.volume_down_rounded, size: 15, color: palette.textTertiary),
        const SizedBox(width: Insets.sm),
        SizedBox(
          width: 64,
          child: Scrubber(
            value: 0.7,
            activeColor: palette.textTertiary,
            trackColor: palette.textTertiary.withValues(alpha: 0.4),
            onChanged: (_) {},
          ),
        ),
      ],
    );
  }
}
