import 'dart:ui';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'artwork.dart';
import 'feedback.dart';
import 'now_playing_page.dart';

/// Compact player pill shown above the tab bar.
class PlayBarPill extends StatelessWidget {
  const PlayBarPill({
    super.key,
    required this.controller,
    this.horizontalPadding = EdgeInsets.zero,
    this.onTap,
  });

  final SosumiController controller;
  final EdgeInsets horizontalPadding;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: horizontalPadding,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(999),
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: 18, sigmaY: 18),
          child: Material(
            color: Colors.transparent,
            child: InkWell(
              onTap: onTap ?? () => showNowPlayingPage(context),
              child: Container(
                decoration: BoxDecoration(
                  color: const Color(0xFF1C1C1E).withValues(alpha: 0.80),
                  border: Border.all(
                    color: Colors.white.withValues(alpha: 0.09),
                  ),
                  borderRadius: BorderRadius.circular(999),
                ),
                padding: const EdgeInsets.fromLTRB(8, 8, 12, 8),
                child: Row(
                  children: [
                    AlbumArt(seed: 7, size: 40, radius: 999),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            'Vodka Cranberry',
                            style: theme.textTheme.titleMedium,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            'Conan Gray',
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
                      icon: const Icon(CupertinoIcons.play_fill),
                    ),
                    IconButton(
                      tooltip: 'Next',
                      onPressed: () => showScaffoldNote(context, 'Next track'),
                      icon: const Icon(CupertinoIcons.forward_end_fill),
                    ),
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

/// Desktop/medium mini player, using the same pill as the mobile accessory.
class MiniPlayer extends StatelessWidget {
  const MiniPlayer({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 10),
      child: PlayBarPill(
        controller: controller,
        horizontalPadding: EdgeInsets.zero,
      ),
    );
  }
}
