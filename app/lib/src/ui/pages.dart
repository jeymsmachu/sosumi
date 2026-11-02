import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'feedback.dart';
import 'now_playing.dart';

class _ShelfItem {
  const _ShelfItem(this.title, this.subtitle, this.icon, this.color);

  final String title;
  final String subtitle;
  final IconData icon;
  final Color color;
}

const _recentlyPlayed = <_ShelfItem>[
  _ShelfItem('Album', 'Last week', Icons.album_rounded, Color(0xFF9B7BFF)),
  _ShelfItem(
    'Playlist',
    'Made for you',
    Icons.queue_music_rounded,
    Color(0xFF58C7FF),
  ),
  _ShelfItem('Single', 'Fresh', Icons.music_note_rounded, Color(0xFFFFB454)),
  _ShelfItem('Artist', 'Top tracks', Icons.mic_rounded, Color(0xFF69E3B0)),
];

const _browseShelf = <_ShelfItem>[
  _ShelfItem(
    'New releases',
    'Fresh from the catalog',
    Icons.new_releases_rounded,
    Color(0xFF9B7BFF),
  ),
  _ShelfItem(
    'For you',
    'Personalized picks',
    Icons.person_rounded,
    Color(0xFF58C7FF),
  ),
  _ShelfItem(
    'Charts',
    'What people hear',
    Icons.bar_chart_rounded,
    Color(0xFFFFB454),
  ),
  _ShelfItem(
    'Essentials',
    'Long-form listening',
    Icons.auto_awesome_rounded,
    Color(0xFF69E3B0),
  ),
  _ShelfItem(
    'Party',
    'High energy',
    Icons.celebration_rounded,
    Color(0xFFFF6B9A),
  ),
  _ShelfItem(
    'Focus',
    'Low distraction',
    Icons.center_focus_strong_rounded,
    Color(0xFF58C7FF),
  ),
  _ShelfItem(
    'Chill',
    'Soft textures',
    Icons.nightlight_rounded,
    Color(0xFF9B7BFF),
  ),
  _ShelfItem(
    'Classical',
    'Timeless forms',
    Icons.piano_rounded,
    Color(0xFFFFB454),
  ),
];

const _radioStations = <_ShelfItem>[
  _ShelfItem(
    'Evening shift',
    'A soft end to the day',
    Icons.wb_twilight_rounded,
    Color(0xFF58C7FF),
  ),
  _ShelfItem(
    'Late traffic',
    'Slow motion, clear head',
    Icons.traffic_rounded,
    Color(0xFF9B7BFF),
  ),
  _ShelfItem(
    'Sunday sheets',
    'Warm, low-key sounds',
    Icons.bed_rounded,
    Color(0xFFFFB454),
  ),
];

const _libraryItems = <_ShelfItem>[
  _ShelfItem(
    'Albums',
    'Everything you own',
    Icons.album_rounded,
    Color(0xFF9B7BFF),
  ),
  _ShelfItem(
    'Artists',
    'People you follow',
    Icons.people_rounded,
    Color(0xFF58C7FF),
  ),
  _ShelfItem(
    'Playlists',
    'Your collections',
    Icons.playlist_play_rounded,
    Color(0xFF69E3B0),
  ),
  _ShelfItem(
    'Songs',
    'The long tail',
    Icons.music_note_rounded,
    Color(0xFFFFB454),
  ),
];

/// Home scaffold: hero, recent activity, and browse cards.
class HomePage extends StatelessWidget {
  const HomePage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(24, 20, 24, 28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _HeroCard(accent: controller.accent),
          const SizedBox(height: 30),
          const _SectionHeader(title: 'Recently played', action: 'See all'),
          const SizedBox(height: 14),
          SizedBox(
            height: 198,
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              itemCount: _recentlyPlayed.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) {
                final item = _recentlyPlayed[index];
                return SizedBox(width: 172, child: _ShelfCard(item: item));
              },
            ),
          ),
          const SizedBox(height: 30),
          const _SectionHeader(title: 'Browse', action: 'See all'),
          const SizedBox(height: 14),
          _shelfGrid(_browseShelf),
        ],
      ),
    );
  }
}

/// Browse scaffold: a responsive catalog grid.
class BrowsePage extends StatelessWidget {
  const BrowsePage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(24, 24, 24, 28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Browse', style: theme.textTheme.headlineMedium),
          const SizedBox(height: 6),
          Text(
            'A responsive view over the same catalog the app will stream later.',
            style: theme.textTheme.bodyMedium,
          ),
          const SizedBox(height: 22),
          _shelfGrid(_browseShelf),
        ],
      ),
    );
  }
}

/// Radio scaffold: station list with the same visual language.
class RadioPage extends StatelessWidget {
  const RadioPage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(24, 24, 24, 28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Radio', style: theme.textTheme.headlineMedium),
          const SizedBox(height: 6),
          Text(
            'Stations will stream once playback is connected.',
            style: theme.textTheme.bodyMedium,
          ),
          const SizedBox(height: 22),
          for (final item in _radioStations) ...[
            _StationRow(item: item),
            const SizedBox(height: 12),
          ],
        ],
      ),
    );
  }
}

/// Library scaffold: local representation of the user's Apple Music library.
class LibraryPage extends StatelessWidget {
  const LibraryPage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(24, 24, 24, 28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Library', style: theme.textTheme.headlineMedium),
          const SizedBox(height: 6),
          Text(
            'This view will read your library through the Apple Music API.',
            style: theme.textTheme.bodyMedium,
          ),
          const SizedBox(height: 22),
          for (final item in _libraryItems) ...[
            _LibraryRow(item: item),
            const SizedBox(height: 12),
          ],
        ],
      ),
    );
  }
}

class _HeroCard extends StatelessWidget {
  const _HeroCard({required this.accent});

  final Color accent;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(28),
        onTap: () => showScaffoldNote(context, 'Catalog'),
        child: Container(
          padding: const EdgeInsets.all(26),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(28),
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [
                accent.withValues(alpha: 0.34),
                const Color(0xFF15151D),
                const Color(0xFF0D0D13),
              ],
              stops: const [0, 0.56, 1],
            ),
            border: Border.all(color: Colors.white.withValues(alpha: 0.09)),
          ),
          child: Stack(
            children: [
              Positioned(
                right: -46,
                top: -58,
                child: Icon(
                  Icons.album_rounded,
                  size: 240,
                  color: Colors.white.withValues(alpha: 0.05),
                ),
              ),
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'A quieter way to listen.',
                    style: theme.textTheme.headlineMedium,
                  ),
                  const SizedBox(height: 10),
                  ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 620),
                    child: Text(
                      'sosumi brings your Apple Music library into one native-feeling app across desktop, tablet, and TV.',
                      style: theme.textTheme.bodyMedium,
                    ),
                  ),
                  const SizedBox(height: 20),
                  Wrap(
                    spacing: 10,
                    runSpacing: 10,
                    children: [
                      FilledButton.icon(
                        onPressed: () =>
                            showScaffoldNote(context, 'Apple Music sign-in'),
                        icon: const Icon(Icons.login_rounded),
                        label: const Text('Connect Apple Music'),
                      ),
                      OutlinedButton(
                        onPressed: () => showScaffoldNote(context, 'Account'),
                        child: const Text('Account'),
                      ),
                    ],
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.title, this.action});

  final String title;
  final String? action;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        Expanded(child: Text(title, style: theme.textTheme.titleLarge)),
        if (action != null)
          TextButton(
            onPressed: () => showScaffoldNote(context, action!),
            child: Text(action!),
          ),
      ],
    );
  }
}

class _ShelfCard extends StatelessWidget {
  const _ShelfCard({required this.item});

  final _ShelfItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: () => showScaffoldNote(context, 'Catalog'),
        child: Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(20),
            color: const Color(0xFF12121A),
            border: Border.all(color: Colors.white.withValues(alpha: 0.07)),
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [
                item.color.withValues(alpha: 0.18),
                const Color(0xFF12121A),
              ],
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Center(
                  child: Icon(
                    item.icon,
                    size: 40,
                    color: item.color.withValues(alpha: 0.88),
                  ),
                ),
              ),
              Text(
                item.title,
                style: theme.textTheme.titleMedium,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 3),
              Text(
                item.subtitle,
                style: theme.textTheme.bodySmall,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StationRow extends StatelessWidget {
  const _StationRow({required this.item});

  final _ShelfItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: () => showScaffoldNote(context, 'Station playback'),
        child: Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(20),
            color: const Color(0xFF12121A),
            border: Border.all(color: Colors.white.withValues(alpha: 0.07)),
          ),
          child: Row(
            children: [
              ArtworkTile(size: 52, accent: item.color, radius: 14),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      item.title,
                      style: theme.textTheme.titleMedium,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      item.subtitle,
                      style: theme.textTheme.bodySmall,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right_rounded),
            ],
          ),
        ),
      ),
    );
  }
}

class _LibraryRow extends StatelessWidget {
  const _LibraryRow({required this.item});

  final _ShelfItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: () => showScaffoldNote(context, 'Library'),
        child: Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(20),
            color: const Color(0xFF12121A),
            border: Border.all(color: Colors.white.withValues(alpha: 0.07)),
          ),
          child: Row(
            children: [
              ArtworkTile(size: 52, accent: item.color, radius: 14),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      item.title,
                      style: theme.textTheme.titleMedium,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      item.subtitle,
                      style: theme.textTheme.bodySmall,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right_rounded),
            ],
          ),
        ),
      ),
    );
  }
}

Widget _shelfGrid(List<_ShelfItem> items) {
  return LayoutBuilder(
    builder: (context, constraints) {
      final width = constraints.maxWidth;
      final columns = width < 560
          ? 2
          : width < 860
          ? 3
          : width < 1200
          ? 4
          : 5;

      return GridView.builder(
        shrinkWrap: true,
        physics: const NeverScrollableScrollPhysics(),
        padding: EdgeInsets.zero,
        gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
          crossAxisCount: columns,
          mainAxisSpacing: 12,
          crossAxisSpacing: 12,
          childAspectRatio: 1.25,
        ),
        itemCount: items.length,
        itemBuilder: (context, index) => _ShelfCard(item: items[index]),
      );
    },
  );
}
