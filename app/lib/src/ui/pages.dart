import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'artwork.dart';
import 'feedback.dart';
import 'settings_page.dart';

/// Home scaffold: large title, top picks, recently played.
class HomePage extends StatelessWidget {
  const HomePage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(0, 8, 0, 140),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PageHeader(title: 'Home', controller: controller),
          const SizedBox(height: 18),
          const _SectionTitle('Top Picks for You'),
          const SizedBox(height: 12),
          SizedBox(
            height: 268,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              scrollDirection: Axis.horizontal,
              itemCount: _topPicks.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) {
                final item = _topPicks[index];
                return _TopPickCard(item: item);
              },
            ),
          ),
          const SizedBox(height: 30),
          const _SectionTitle('Recently Played'),
          const SizedBox(height: 12),
          SizedBox(
            height: 204,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              scrollDirection: Axis.horizontal,
              itemCount: _recentlyPlayed.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) {
                final item = _recentlyPlayed[index];
                return _AlbumCard(item: item);
              },
            ),
          ),
          const SizedBox(height: 30),
          const _SectionTitle('Made for You'),
          const SizedBox(height: 12),
          SizedBox(
            height: 204,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              scrollDirection: Axis.horizontal,
              itemCount: _madeForYou.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) {
                final item = _madeForYou[index];
                return _AlbumCard(item: item);
              },
            ),
          ),
        ],
      ),
    );
  }
}

/// New releases scaffold: editorial features and exclusive content.
class NewPage extends StatelessWidget {
  const NewPage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(0, 8, 0, 140),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PageHeader(title: 'New', controller: controller),
          const SizedBox(height: 16),
          const _SectionTitle('Featured'),
          const SizedBox(height: 12),
          SizedBox(
            height: 300,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              scrollDirection: Axis.horizontal,
              itemCount: _featured.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) =>
                  _FeaturedCard(item: _featured[index]),
            ),
          ),
          const SizedBox(height: 30),
          const _SectionTitle('Only on Apple Music'),
          const SizedBox(height: 12),
          SizedBox(
            height: 218,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              scrollDirection: Axis.horizontal,
              itemCount: _exclusives.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) =>
                  _AlbumCard(item: _exclusives[index], width: 190),
            ),
          ),
        ],
      ),
    );
  }
}

/// Radio scaffold: stations with rich gradient art.
class RadioPage extends StatelessWidget {
  const RadioPage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(0, 8, 0, 140),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PageHeader(title: 'Radio', controller: controller),
          const SizedBox(height: 18),
          SizedBox(
            height: 210,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              scrollDirection: Axis.horizontal,
              itemCount: _stations.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) =>
                  _AlbumCard(item: _stations[index], width: 180),
            ),
          ),
          const SizedBox(height: 30),
          for (final item in _stations)
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 0, 20, 10),
              child: _StationRow(item: item),
            ),
        ],
      ),
    );
  }
}

/// Library scaffold: favourite songs and collection rows.
class LibraryPage extends StatelessWidget {
  const LibraryPage({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(0, 8, 0, 140),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PageHeader(title: 'Library', controller: controller),
          const SizedBox(height: 16),
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 20),
            child: _FavoriteSongsCard(),
          ),
          const SizedBox(height: 24),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: Column(
              children: [
                for (final item in _libraryRows) _LibraryListRow(item: item),
              ],
            ),
          ),
          const SizedBox(height: 28),
          const _SectionTitle('Recently Added'),
          const SizedBox(height: 12),
          SizedBox(
            height: 210,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              scrollDirection: Axis.horizontal,
              itemCount: _recentlyAdded.length,
              separatorBuilder: (_, _) => const SizedBox(width: 12),
              itemBuilder: (context, index) =>
                  _AlbumCard(item: _recentlyAdded[index], width: 168),
            ),
          ),
        ],
      ),
    );
  }
}

/// Search scaffold: browse categories and a working local filter.
class SearchPage extends StatefulWidget {
  const SearchPage({super.key, required this.controller});

  final SosumiController controller;

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final TextEditingController _query = TextEditingController();
  String _filter = '';

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final query = _filter.trim().toLowerCase();
    final categories = query.isEmpty
        ? _categories
        : _categories
              .where((category) => category.name.toLowerCase().contains(query))
              .toList();

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(0, 8, 0, 140),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PageHeader(title: 'Search', controller: widget.controller),
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 14, 20, 0),
            child: TextField(
              controller: _query,
              onChanged: (value) => setState(() => _filter = value),
              decoration: InputDecoration(
                hintText: 'Artists, Songs, Lyrics and more',
                prefixIcon: const Icon(CupertinoIcons.search),
                suffixIcon: _filter.isEmpty
                    ? null
                    : IconButton(
                        tooltip: 'Clear',
                        onPressed: () {
                          _query.clear();
                          setState(() => _filter = '');
                        },
                        icon: const Icon(CupertinoIcons.xmark_circle_fill),
                      ),
              ),
            ),
          ),
          const SizedBox(height: 22),
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 12),
            child: Text('Browse Categories', style: theme.textTheme.titleLarge),
          ),
          LayoutBuilder(
            builder: (context, constraints) {
              final width = constraints.maxWidth;
              final columns = width < 560
                  ? 2
                  : width < 880
                  ? 3
                  : width < 1200
                  ? 4
                  : 5;

              return GridView.builder(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                padding: const EdgeInsets.symmetric(horizontal: 20),
                gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                  crossAxisCount: columns,
                  crossAxisSpacing: 12,
                  mainAxisSpacing: 12,
                  childAspectRatio: 1.55,
                ),
                itemCount: categories.length,
                itemBuilder: (context, index) =>
                    _GenreCard(category: categories[index]),
              );
            },
          ),
        ],
      ),
    );
  }
}

// ── Data ─────────────────────────────────────────────────────────────────────

class _CardItem {
  const _CardItem(this.title, this.subtitle, this.seed, {this.badge});

  final String title;
  final String subtitle;
  final int seed;
  final String? badge;
}

const _topPicks = <_CardItem>[
  _CardItem('The Weeknd Essentials', 'Apple Music R&B', 1, badge: 'NEW SINGLE'),
  _CardItem('Global Hits', 'Apple Music Pop', 2),
  _CardItem('Latin Heat', 'Apple Music Latino', 3),
  _CardItem('Late Night Keys', 'Apple Music Jazz', 4),
];

const _recentlyPlayed = <_CardItem>[
  _CardItem('Vodka Cranberry', 'Conan Gray', 7),
  _CardItem('Skye Newman Live', 'Apple Music Live', 8),
  _CardItem('Lady of the Lake', 'Maya Lin', 9),
  _CardItem('Neon Hours', 'Rina Sawayama', 10),
  _CardItem('Static Bloom', 'The Beths', 11),
  _CardItem('Half Light', 'Keshi', 12),
];

const _madeForYou = <_CardItem>[
  _CardItem('Discover Mix', 'Made for you', 13),
  _CardItem('Chill Mix', 'Made for you', 14),
  _CardItem('New Music Mix', 'Updated Friday', 15),
  _CardItem('Favorites Mix', 'Made for you', 16),
  _CardItem('Replay 2026', 'Your top songs', 17),
];

const _featured = <_CardItem>[
  _CardItem('The Weeknd Essentials', 'Apple Music R&B', 1, badge: 'NEW SINGLE'),
  _CardItem('Top Picks', 'Apple Music', 18, badge: 'UPDATED'),
  _CardItem('Pop Hits', 'Apple Music Pop', 19, badge: 'NEW'),
];

const _exclusives = <_CardItem>[
  _CardItem('Zara Larsson Sessions', 'Zara Larsson', 20, badge: 'SESSIONS'),
  _CardItem('Apple Music Live', 'Skye Newman', 8, badge: 'LIVE'),
  _CardItem('Lady of the Lake', 'Maya Lin', 9),
  _CardItem('Studio Sessions', 'Nia Archives', 21),
];

const _stations = <_CardItem>[
  _CardItem('Apple Music 1', 'Global Pop', 22),
  _CardItem('Apple Music Hits', 'All-time favourites', 23),
  _CardItem('Apple Music Country', 'Country hits', 24),
  _CardItem('Chill Mix', 'Lo-fi & ambient', 14),
  _CardItem('Workout Mix', 'High energy', 25),
  _CardItem('Late Night', 'R&B & soul', 26),
];

const _libraryRows = <_CardItem>[
  _CardItem('Playlists', 'Your collections', 27),
  _CardItem('Artists', 'People you follow', 28),
  _CardItem('Albums', 'Everything you own', 29),
  _CardItem('Songs', 'The long tail', 30),
  _CardItem('Downloaded', 'Available offline', 31),
  _CardItem('TV & Movies', 'Video library', 32),
];

const _recentlyAdded = <_CardItem>[
  _CardItem('Vodka Cranberry', 'Conan Gray', 7),
  _CardItem('Patrick Gimena II', 'Playlist', 33),
  _CardItem('Skye Newman Live', 'Apple Music Live', 8),
  _CardItem('Lady of the Lake', 'Maya Lin', 9),
  _CardItem('Static Bloom', 'The Beths', 11),
];

class _BrowseCategory {
  const _BrowseCategory(this.name, this.seed);

  final String name;
  final int seed;
}

const _categories = <_BrowseCategory>[
  _BrowseCategory('Apple Music Radio', 22),
  _BrowseCategory('K-Pop', 34),
  _BrowseCategory('Alternative', 35),
  _BrowseCategory('Pop', 36),
  _BrowseCategory('Original Pilipino Music', 37),
  _BrowseCategory('Hip-Hop/Rap', 38),
  _BrowseCategory('R&B', 39),
  _BrowseCategory('Rock', 40),
  _BrowseCategory('Film, TV & Stage', 41),
  _BrowseCategory('Charts', 42),
  _BrowseCategory('Jazz', 43),
  _BrowseCategory('Country', 44),
];

// ── Shared widgets ───────────────────────────────────────────────────────────

class _PageHeader extends StatelessWidget {
  const _PageHeader({required this.title, required this.controller});

  final String title;
  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 10, 16, 0),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: theme.textTheme.headlineMedium?.copyWith(fontSize: 38),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 12),
          CircleAvatar(
            radius: 18,
            backgroundColor: Colors.white.withValues(alpha: 0.09),
            child: Text(
              'S',
              style: theme.textTheme.labelLarge?.copyWith(
                color: Colors.white.withValues(alpha: 0.88),
              ),
            ),
          ),
          const SizedBox(width: 4),
          IconButton(
            tooltip: 'More',
            onPressed: () => showSettingsPage(context),
            icon: const Icon(CupertinoIcons.ellipsis),
          ),
        ],
      ),
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.title);

  final String title;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Text(title, style: theme.textTheme.headlineSmall),
      ),
    );
  }
}

class _TopPickCard extends StatelessWidget {
  const _TopPickCard({required this.item});

  final _CardItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(18),
        onTap: () => showScaffoldNote(context, 'Catalog'),
        child: SizedBox(
          width: 236,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: AlbumArt(
                  seed: item.seed,
                  radius: 18,
                  overlay: item.badge == null
                      ? null
                      : Padding(
                          padding: const EdgeInsets.all(10),
                          child: _OverlayLabel(item.badge!),
                        ),
                ),
              ),
              const SizedBox(height: 10),
              Text(
                item.title,
                style: theme.textTheme.titleMedium,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 2),
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

class _FeaturedCard extends StatelessWidget {
  const _FeaturedCard({required this.item});

  final _CardItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return SizedBox(
      width: 300,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(
              item.badge?.toUpperCase() ?? 'FEATURED',
              style: theme.textTheme.labelSmall?.copyWith(
                color: Colors.white.withValues(alpha: 0.56),
                fontWeight: FontWeight.w700,
                letterSpacing: 1.1,
              ),
            ),
          ),
          Text(item.title, style: theme.textTheme.headlineSmall),
          Text(item.subtitle, style: theme.textTheme.bodyMedium),
          const SizedBox(height: 10),
          Expanded(child: AlbumArt(seed: item.seed, radius: 16)),
        ],
      ),
    );
  }
}

class _AlbumCard extends StatelessWidget {
  const _AlbumCard({required this.item, this.width = 158});

  final _CardItem item;
  final double width;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: () => showScaffoldNote(context, 'Catalog'),
        child: SizedBox(
          width: width,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: AlbumArt(
                  seed: item.seed,
                  radius: 16,
                  overlay: item.badge == null
                      ? null
                      : Padding(
                          padding: const EdgeInsets.all(10),
                          child: _OverlayLabel(item.badge!),
                        ),
                ),
              ),
              const SizedBox(height: 8),
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
      ),
    );
  }
}

class _GenreCard extends StatelessWidget {
  const _GenreCard({required this.category});

  final _BrowseCategory category;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: () => showScaffoldNote(context, category.name),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          child: Stack(
            fit: StackFit.expand,
            children: [
              AlbumArt(seed: category.seed, radius: 16),
              Align(
                alignment: Alignment.bottomLeft,
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Text(
                    category.name,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w800,
                      color: Colors.white,
                    ),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _FavoriteSongsCard extends StatelessWidget {
  const _FavoriteSongsCard();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(18),
        onTap: () => showScaffoldNote(context, 'Favourite Songs'),
        child: SizedBox(
          width: 188,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AspectRatio(
                aspectRatio: 1,
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(18),
                  child: Container(
                    color: const Color(0xFFF2F2F2),
                    child: Center(
                      child: Icon(
                        CupertinoIcons.star_fill,
                        size: 78,
                        color: const Color(0xFFFA2D48),
                      ),
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Favourite Songs',
                style: theme.textTheme.bodyMedium,
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

class _LibraryListRow extends StatelessWidget {
  const _LibraryListRow({required this.item});

  final _CardItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = switch (item.title) {
      'Playlists' => CupertinoIcons.music_note_list,
      'Artists' => CupertinoIcons.person_2,
      'Albums' => CupertinoIcons.music_albums,
      'Songs' => CupertinoIcons.music_note,
      'Downloaded' => CupertinoIcons.arrow_down_circle,
      _ => CupertinoIcons.tv,
    };

    return InkWell(
      onTap: () => showScaffoldNote(context, item.title),
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 14),
            child: Row(
              children: [
                Icon(icon, size: 22, color: const Color(0xFFFA2D48)),
                const SizedBox(width: 16),
                Expanded(
                  child: Text(item.title, style: theme.textTheme.titleMedium),
                ),
                Icon(
                  CupertinoIcons.chevron_forward,
                  size: 16,
                  color: Colors.white.withValues(alpha: 0.36),
                ),
              ],
            ),
          ),
          Divider(
            height: 1,
            thickness: 0.5,
            indent: 38,
            color: Colors.white.withValues(alpha: 0.10),
          ),
        ],
      ),
    );
  }
}

class _StationRow extends StatelessWidget {
  const _StationRow({required this.item});

  final _CardItem item;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: () => showScaffoldNote(context, 'Station playback'),
        child: Row(
          children: [
            AlbumArt(seed: item.seed, size: 72, radius: 14),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(item.title, style: theme.textTheme.titleMedium),
                  Text(item.subtitle, style: theme.textTheme.bodySmall),
                ],
              ),
            ),
            IconButton(
              tooltip: 'Play',
              onPressed: () => showScaffoldNote(context, 'Playback'),
              icon: const Icon(CupertinoIcons.play_fill),
            ),
          ],
        ),
      ),
    );
  }
}

class _OverlayLabel extends StatelessWidget {
  const _OverlayLabel(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.42),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Text(
          text.toUpperCase(),
          style: Theme.of(context).textTheme.labelSmall?.copyWith(
            color: Colors.white,
            fontWeight: FontWeight.w800,
            letterSpacing: 0.8,
          ),
        ),
      ),
    );
  }
}
