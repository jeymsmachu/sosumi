import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'artwork.dart';

/// Opens the full-screen player.
Future<void> showNowPlayingPage(BuildContext context) {
  return Navigator.of(
    context,
    rootNavigator: true,
  ).push(MaterialPageRoute<void>(builder: (_) => const NowPlayingPage()));
}

/// Full-screen player, modeled on Apple Music's Android/iOS presentation.
class NowPlayingPage extends StatefulWidget {
  const NowPlayingPage({super.key});

  @override
  State<NowPlayingPage> createState() => _NowPlayingPageState();
}

class _NowPlayingPageState extends State<NowPlayingPage> {
  bool _showLyrics = false;

  @override
  Widget build(BuildContext context) {
    final controller = SosumiScope.of(context);
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.black,
      body: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 8, 8, 4),
              child: Row(
                children: [
                  IconButton(
                    tooltip: 'Close player',
                    onPressed: () => Navigator.of(context).maybePop(),
                    icon: const Icon(CupertinoIcons.chevron_down),
                  ),
                  const Spacer(),
                  Text(
                    'Playing from Apple Music',
                    style: theme.textTheme.labelMedium?.copyWith(
                      color: Colors.white.withValues(alpha: 0.62),
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const Spacer(),
                  IconButton(
                    tooltip: 'More',
                    onPressed: () {},
                    icon: const Icon(CupertinoIcons.ellipsis),
                  ),
                ],
              ),
            ),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20),
                child: AnimatedSwitcher(
                  duration: const Duration(milliseconds: 280),
                  child: _showLyrics
                      ? _LyricsView(
                          key: const ValueKey('lyrics'),
                          accent: controller.accent,
                        )
                      : Center(
                          key: const ValueKey('artwork'),
                          child: AlbumArt(seed: 7, radius: 14, overlay: null),
                        ),
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 10, 20, 10),
              child: Column(
                children: [
                  Text(
                    'Vodka Cranberry',
                    style: theme.textTheme.headlineSmall,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  Text(
                    'Conan Gray',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: controller.accent,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 12),
                  SliderTheme(
                    data: SliderTheme.of(context).copyWith(
                      trackHeight: 4,
                      thumbShape: const RoundSliderThumbShape(
                        enabledThumbRadius: 7,
                      ),
                      overlayShape: const RoundSliderOverlayShape(
                        overlayRadius: 14,
                      ),
                    ),
                    child: Slider(value: 0.34, onChanged: (_) {}),
                  ),
                  Row(
                    children: [
                      Text('1:05', style: theme.textTheme.labelSmall),
                      const Spacer(),
                      Text('-1:55', style: theme.textTheme.labelSmall),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      IconButton(
                        tooltip: 'Shuffle',
                        onPressed: () {},
                        icon: const Icon(CupertinoIcons.shuffle),
                      ),
                      IconButton(
                        tooltip: 'Previous',
                        onPressed: () {},
                        icon: const Icon(CupertinoIcons.backward_end_fill),
                      ),
                      SizedBox(
                        width: 62,
                        height: 62,
                        child: FilledButton(
                          onPressed: () {},
                          style: FilledButton.styleFrom(
                            backgroundColor: controller.accent,
                            foregroundColor: Colors.black,
                            shape: const CircleBorder(),
                            padding: EdgeInsets.zero,
                          ),
                          child: const Icon(CupertinoIcons.play_fill, size: 30),
                        ),
                      ),
                      IconButton(
                        tooltip: 'Next',
                        onPressed: () {},
                        icon: const Icon(CupertinoIcons.forward_end_fill),
                      ),
                      IconButton(
                        tooltip: 'Repeat',
                        onPressed: () {},
                        icon: const Icon(CupertinoIcons.repeat),
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      IconButton(
                        tooltip: 'Lyrics',
                        onPressed: () =>
                            setState(() => _showLyrics = !_showLyrics),
                        icon: Icon(
                          CupertinoIcons.text_bubble,
                          color: _showLyrics
                              ? controller.accent
                              : Colors.white.withValues(alpha: 0.78),
                        ),
                      ),
                      IconButton(
                        tooltip: 'Share',
                        onPressed: () {},
                        icon: const Icon(CupertinoIcons.share_up),
                      ),
                      IconButton(
                        tooltip: 'Queue',
                        onPressed: () {},
                        icon: const Icon(CupertinoIcons.list_bullet),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _LyricsView extends StatelessWidget {
  const _LyricsView({super.key, required this.accent});

  final Color accent;

  static const _lines = <String>[
    'Bitter morning, silver light',
    'I keep the radio on low',
    'You said the weather looked like us',
    'But I was somewhere else entirely',
    'And every glass tastes like the last',
    'One you poured for someone else',
    'So I learn to live with that',
    'And call it something sweet',
  ];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Align(
      alignment: Alignment.centerLeft,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (var i = 0; i < _lines.length; i++)
            Padding(
              padding: const EdgeInsets.only(bottom: 14),
              child: Text(
                _lines[i],
                style: i == 3
                    ? theme.textTheme.headlineSmall?.copyWith(
                        color: accent,
                        fontWeight: FontWeight.w800,
                        fontSize: 30,
                        height: 1.1,
                      )
                    : theme.textTheme.headlineSmall?.copyWith(
                        color: Colors.white.withValues(alpha: 0.22),
                        fontWeight: FontWeight.w800,
                        fontSize: 30,
                        height: 1.1,
                      ),
              ),
            ),
        ],
      ),
    );
  }
}
