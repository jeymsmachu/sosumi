import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'feedback.dart';
import 'glass_surface.dart';

/// Placeholder artwork. Real artwork will come from the Apple Music API cache.
class ArtworkTile extends StatelessWidget {
  const ArtworkTile({
    super.key,
    required this.size,
    required this.accent,
    this.radius = 18,
  });

  final double size;
  final Color accent;
  final double radius;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(radius),
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            accent.withValues(alpha: 0.88),
            const Color(0xFF171722),
            const Color(0xFF0C0C11),
          ],
          stops: const [0, 0.62, 1],
        ),
        boxShadow: [
          BoxShadow(
            color: accent.withValues(alpha: 0.18),
            blurRadius: 24,
            offset: const Offset(0, 10),
          ),
        ],
      ),
      child: Stack(
        children: [
          Positioned(
            top: size * 0.16,
            right: size * 0.12,
            child: Icon(
              Icons.graphic_eq_rounded,
              size: size * 0.42,
              color: Colors.white.withValues(alpha: 0.82),
            ),
          ),
          Positioned(
            left: size * 0.12,
            bottom: size * 0.12,
            child: Text(
              'sosumi',
              style: TextStyle(
                color: Colors.white.withValues(alpha: 0.72),
                fontSize: size * 0.10,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.4,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Full now-playing surface for the expanded layout.
class NowPlayingPanel extends StatelessWidget {
  const NowPlayingPanel({
    super.key,
    required this.controller,
    this.compact = false,
    this.embedded = false,
  });

  final SosumiController controller;
  final bool compact;
  final bool embedded;

  @override
  Widget build(BuildContext context) {
    final content = _NowPlayingContent(
      controller: controller,
      compact: compact,
    );

    if (embedded) return content;

    return Padding(
      padding: const EdgeInsets.fromLTRB(0, 16, 16, 16),
      child: SizedBox(
        width: 328,
        child: GlassSurface(
          enabled: controller.glassEnabled,
          radius: 30,
          padding: const EdgeInsets.all(20),
          child: content,
        ),
      ),
    );
  }
}

class _NowPlayingContent extends StatelessWidget {
  const _NowPlayingContent({required this.controller, required this.compact});

  final SosumiController controller;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final accent = controller.accent;
    final artworkSize = compact ? 220.0 : null;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Text(
              'NOW PLAYING',
              style: theme.textTheme.labelSmall?.copyWith(
                color: Colors.white.withValues(alpha: 0.48),
                letterSpacing: 1.6,
                fontWeight: FontWeight.w700,
              ),
            ),
            const Spacer(),
            IconButton(
              tooltip: 'Queue',
              onPressed: () => showScaffoldNote(context, 'Queue'),
              icon: const Icon(Icons.queue_music_rounded),
            ),
          ],
        ),
        SizedBox(height: compact ? 10 : 6),
        if (artworkSize == null)
          AspectRatio(
            aspectRatio: 1,
            child: ArtworkTile(size: 260, accent: accent, radius: 24),
          )
        else
          Center(
            child: ArtworkTile(size: artworkSize, accent: accent, radius: 24),
          ),
        const SizedBox(height: 18),
        Text(
          'Nothing playing',
          style: theme.textTheme.headlineSmall,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 4),
        Text(
          'Connect Apple Music to begin.',
          style: theme.textTheme.bodyMedium,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 18),
        ClipRRect(
          borderRadius: BorderRadius.circular(999),
          child: LinearProgressIndicator(
            value: 0,
            minHeight: 4,
            backgroundColor: Colors.white.withValues(alpha: 0.10),
            color: accent,
          ),
        ),
        const SizedBox(height: 6),
        Row(
          children: [
            Text('0:00', style: theme.textTheme.labelSmall),
            const Spacer(),
            Text('--:--', style: theme.textTheme.labelSmall),
          ],
        ),
        const SizedBox(height: 6),
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            IconButton(
              tooltip: 'Shuffle',
              onPressed: () => showScaffoldNote(context, 'Shuffle'),
              icon: const Icon(Icons.shuffle_rounded),
            ),
            IconButton(
              tooltip: 'Previous',
              onPressed: () => showScaffoldNote(context, 'Previous track'),
              icon: const Icon(Icons.skip_previous_rounded),
            ),
            SizedBox(
              width: 54,
              height: 54,
              child: FilledButton(
                onPressed: () => showScaffoldNote(context, 'Playback'),
                style: FilledButton.styleFrom(
                  backgroundColor: accent,
                  foregroundColor: Colors.black,
                  shape: const CircleBorder(),
                  padding: EdgeInsets.zero,
                ),
                child: const Icon(Icons.play_arrow_rounded, size: 30),
              ),
            ),
            IconButton(
              tooltip: 'Next',
              onPressed: () => showScaffoldNote(context, 'Next track'),
              icon: const Icon(Icons.skip_next_rounded),
            ),
            IconButton(
              tooltip: 'Repeat',
              onPressed: () => showScaffoldNote(context, 'Repeat'),
              icon: const Icon(Icons.repeat_rounded),
            ),
          ],
        ),
        const SizedBox(height: 12),
        const Center(
          child: StatusPill(
            label: 'Not connected',
            icon: Icons.link_off_rounded,
            color: Colors.white70,
          ),
        ),
        const SizedBox(height: 12),
        FilledButton.tonalIcon(
          onPressed: () => showScaffoldNote(context, 'Apple Music sign-in'),
          icon: const Icon(Icons.login_rounded),
          label: const Text('Connect Apple Music'),
        ),
      ],
    );
  }
}

/// Compact now-playing bar shown below the content on narrower layouts.
class MiniPlayer extends StatelessWidget {
  const MiniPlayer({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      child: GlassSurface(
        enabled: controller.glassEnabled,
        radius: 20,
        padding: EdgeInsets.zero,
        child: InkWell(
          borderRadius: BorderRadius.circular(20),
          onTap: () => showNowPlayingSheet(context, controller),
          child: Padding(
            padding: const EdgeInsets.all(9),
            child: Row(
              children: [
                ArtworkTile(size: 44, accent: controller.accent, radius: 12),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        'Nothing playing',
                        style: theme.textTheme.titleMedium,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        'Not connected',
                        style: theme.textTheme.bodySmall,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                IconButton(
                  tooltip: 'Play',
                  onPressed: () => showScaffoldNote(context, 'Playback'),
                  icon: const Icon(Icons.play_arrow_rounded),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Opens the expanded now-playing surface as a bottom sheet.
Future<void> showNowPlayingSheet(
  BuildContext context,
  SosumiController controller,
) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: Colors.transparent,
    barrierColor: Colors.black.withValues(alpha: 0.55),
    builder: (context) {
      return Padding(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        child: Align(
          alignment: Alignment.bottomCenter,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: GlassSurface(
              enabled: controller.glassEnabled,
              radius: 30,
              padding: const EdgeInsets.all(20),
              child: NowPlayingPanel(
                controller: controller,
                compact: true,
                embedded: true,
              ),
            ),
          ),
        ),
      );
    },
  );
}
