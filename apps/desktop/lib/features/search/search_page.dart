import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/player.dart';
import '../../rust/api.dart';
import '../../theme/theme.dart';
import 'search_controller.dart';

/// The search page.
///
/// One field, then grouped results: songs first, because that is what a search
/// usually means, then albums and artists. Selecting a song plays it.
class SearchPage extends ConsumerStatefulWidget {
  const SearchPage({super.key});

  @override
  ConsumerState<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends ConsumerState<SearchPage> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final search = ref.watch(searchProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Center(
          child: ConstrainedBox(
            // Apple caps a search field to a comfortable reading measure rather
            // than letting it span the whole window.
            constraints: const BoxConstraints(maxWidth: 560),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(
                Insets.page,
                0,
                Insets.page,
                Insets.lg,
              ),
              child: _SearchField(
                controller: _controller,
                onChanged: (value) =>
                    ref.read(searchProvider.notifier).search(value),
                onClear: () {
                  _controller.clear();
                  ref.read(searchProvider.notifier).clear();
                },
              ),
            ),
          ),
        ),
        Expanded(child: _Results(state: search)),
      ],
    );
  }
}

class _SearchField extends StatelessWidget {
  const _SearchField({
    required this.controller,
    required this.onChanged,
    required this.onClear,
  });

  final TextEditingController controller;
  final ValueChanged<String> onChanged;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    // A search field is content-layer, so the fill is a standard surface rather
    // than glass: it sits in the scrollable area, not above it.
    return Container(
      decoration: BoxDecoration(
        color: palette.fill,
        borderRadius: BorderRadius.circular(Radii.row),
        border: Border.all(color: palette.separator, width: 0.5),
      ),
      padding: const EdgeInsets.symmetric(horizontal: Insets.md),
      child: Row(
        children: [
          Icon(Icons.search_rounded, size: 16, color: palette.textSecondary),
          const SizedBox(width: Insets.sm),
          Expanded(
            child: TextField(
              controller: controller,
              onChanged: onChanged,
              style: TextStyles.body.copyWith(color: palette.textPrimary),
              cursorColor: palette.accent,
              decoration: InputDecoration(
                border: InputBorder.none,
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(vertical: Insets.md),
                hintText: 'Songs, albums, artists',
                hintStyle: TextStyles.body.copyWith(color: palette.textSecondary),
              ),
            ),
          ),
          if (controller.text.isNotEmpty)
            InteractiveGlass(
              onTap: onClear,
              radius: Radii.pill,
              padding: const EdgeInsets.all(Insets.xs),
              child: Icon(Icons.close_rounded, size: 16, color: palette.textSecondary),
            ),
        ],
      ),
    );
  }
}

class _Results extends ConsumerWidget {
  const _Results({required this.state});

  final SearchState state;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (state.error != null) {
      return _Placeholder(
        icon: Icons.warning_amber_rounded,
        message: state.error!,
      );
    }
    if (state.query.isEmpty) {
      return const _Placeholder(
        icon: Icons.search_rounded,
        message: 'Search for something to listen to',
      );
    }
    if (state.loading && !state.hasResults) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (!state.hasResults) {
      return _Placeholder(
        icon: Icons.search_off_rounded,
        message: 'No results for "${state.query}"',
      );
    }

    final results = state.results!;
    return ListView(
      // Bottom padding clears the floating player, which overlays the content.
      padding: const EdgeInsets.fromLTRB(
        Insets.page,
        0,
        Insets.page,
        Sizes.playerHeight + Sizes.playerInset * 2,
      ),
      children: [
        if (results.tracks.isNotEmpty) ...[
          const _SectionHeading('Songs'),
          for (final track in results.tracks)
            _TrackRow(
              track: track,
              onTap: () => ref.read(playerProvider.notifier).play(track.id),
            ),
        ],
        if (results.albums.isNotEmpty) ...[
          const _SectionHeading('Albums'),
          for (final album in results.albums) _AlbumRow(album: album),
        ],
        if (results.artists.isNotEmpty) ...[
          const _SectionHeading('Artists'),
          for (final artist in results.artists) _ArtistRow(artist: artist),
        ],
      ],
    );
  }
}

class _SectionHeading extends StatelessWidget {
  const _SectionHeading(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: Insets.lg, bottom: Insets.sm),
      child: Text(
        label,
        style: TextStyles.headline.copyWith(color: context.palette.textPrimary),
      ),
    );
  }
}

class _TrackRow extends StatelessWidget {
  const _TrackRow({required this.track, required this.onTap});

  final TrackDto track;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return _ResultRow(
      artworkUrl: track.artworkUrl,
      title: track.title,
      subtitle: track.artist,
      onTap: onTap,
      round: false,
    );
  }
}

class _AlbumRow extends StatelessWidget {
  const _AlbumRow({required this.album});

  final AlbumDto album;

  @override
  Widget build(BuildContext context) {
    return _ResultRow(
      artworkUrl: album.artworkUrl,
      title: album.title,
      subtitle: album.artist,
      round: false,
    );
  }
}

class _ArtistRow extends StatelessWidget {
  const _ArtistRow({required this.artist});

  final ArtistDto artist;

  @override
  Widget build(BuildContext context) {
    return _ResultRow(
      artworkUrl: artist.artworkUrl,
      title: artist.name,
      subtitle: 'Artist',
      round: true,
    );
  }
}

class _ResultRow extends StatefulWidget {
  const _ResultRow({
    required this.artworkUrl,
    required this.title,
    required this.subtitle,
    required this.round,
    this.onTap,
  });

  final String? artworkUrl;
  final String title;
  final String subtitle;
  final bool round;
  final VoidCallback? onTap;

  @override
  State<_ResultRow> createState() => _ResultRowState();
}

class _ResultRowState extends State<_ResultRow> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        behavior: HitTestBehavior.opaque,
        child: AnimatedContainer(
          duration: Motion.fast,
          padding: const EdgeInsets.symmetric(vertical: Insets.sm, horizontal: Insets.sm),
          decoration: BoxDecoration(
            color: _hovered ? palette.glassFill : Colors.transparent,
            borderRadius: BorderRadius.circular(Radii.row),
          ),
          child: Row(
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(widget.round ? Radii.pill : Radii.small),
                child: SizedBox(
                  width: Sizes.listArtwork,
                  height: Sizes.listArtwork,
                  child: widget.artworkUrl == null
                      ? ColoredBox(
                          color: palette.glassFill,
                          child: Icon(
                            Icons.music_note_rounded,
                            size: 16,
                            color: palette.textSecondary,
                          ),
                        )
                      : Image.network(
                          widget.artworkUrl!,
                          fit: BoxFit.cover,
                          errorBuilder: (_, _, _) => ColoredBox(
                            color: palette.glassFill,
                            child: Icon(
                              Icons.music_note_rounded,
                              size: 16,
                              color: palette.textSecondary,
                            ),
                          ),
                        ),
                ),
              ),
              const SizedBox(width: Insets.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      widget.title,
                      style: TextStyles.rowTitle.copyWith(color: palette.textPrimary),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      widget.subtitle,
                      style: TextStyles.callout.copyWith(color: palette.textSecondary),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Placeholder extends StatelessWidget {
  const _Placeholder({required this.icon, required this.message});

  final IconData icon;
  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 32, color: palette.textSecondary.withValues(alpha: 0.6)),
          const SizedBox(height: Insets.md),
          Text(message, style: TextStyles.body.copyWith(color: palette.textSecondary)),
        ],
      ),
    );
  }
}
