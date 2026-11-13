import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/sidebar.dart';
import '../../core/session.dart';
import '../../theme/theme.dart';

/// The user's own music.
///
/// One page serves every Library section, because they differ only in which kind
/// of item they list. `section` selects the filter, which is why the sidebar can
/// offer Recently Added, Artists, Albums, Songs, and Playlists as separate
/// destinations without five near-identical pages.
///
/// When signed out it explains what signing in adds and offers the action,
/// following the HIG's rule to delay sign-in and to state its benefit.
class LibraryPage extends ConsumerWidget {
  const LibraryPage({super.key, required this.section, required this.onSignIn});

  final AppSection section;

  /// Invoked by the signed-out state. The shell owns navigation, so the page
  /// reports the intent rather than deciding where to go.
  final VoidCallback onSignIn;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final signedIn = ref.watch(sessionProvider.select((s) => s.signedIn));

    if (!signedIn) {
      return _SignedOutState(onSignIn: onSignIn);
    }

    return ListView(
      // Bottom padding clears the floating player, which overlays the content.
      padding: const EdgeInsets.fromLTRB(
        Insets.page,
        0,
        Insets.page,
        Sizes.playerHeight + Sizes.playerInset * 2,
      ),
      children: [
        _LibraryFilter(selected: section),
        const SizedBox(height: Insets.xl),
        const _EmptyLibrary(),
      ],
    );
  }
}

/// The Library's kind filter.
///
/// A horizontal control rather than a nested sidebar, because the kinds are
/// peers rather than a hierarchy.
class _LibraryFilter extends StatelessWidget {
  const _LibraryFilter({required this.selected});

  final AppSection selected;

  static const _kinds = [
    AppSection.songs,
    AppSection.albums,
    AppSection.artists,
    AppSection.playlists,
  ];

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: 28,
      child: Row(
        children: [
          for (final (index, kind) in _kinds.indexed) ...[
            if (index > 0) const SizedBox(width: Insets.xs),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: Insets.md),
              alignment: Alignment.center,
              decoration: BoxDecoration(
                color: kind == selected ? palette.selection : Colors.transparent,
                borderRadius: BorderRadius.circular(Radii.small),
              ),
              child: Text(
                kind.label,
                style: TextStyles.body.copyWith(
                  color: kind == selected ? palette.textPrimary : palette.textSecondary,
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _EmptyLibrary extends StatelessWidget {
  const _EmptyLibrary();

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.only(top: Insets.xl * 2),
      child: Column(
        children: [
          Icon(Icons.library_music_outlined, size: 32, color: palette.textTertiary),
          const SizedBox(height: Insets.lg),
          Text(
            'Your library is empty',
            style: TextStyles.body.copyWith(color: palette.textSecondary),
          ),
          const SizedBox(height: Insets.xs),
          Text(
            'Save something from Search to get started.',
            style: TextStyles.subheadline.copyWith(color: palette.textTertiary),
          ),
        ],
      ),
    );
  }
}

class _SignedOutState extends StatelessWidget {
  const _SignedOutState({required this.onSignIn});

  final VoidCallback onSignIn;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 320),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.library_music_outlined, size: 36, color: palette.textTertiary),
            const SizedBox(height: Insets.lg),
            Text('Your Library',
                style: TextStyles.title3.copyWith(color: palette.textPrimary)),
            const SizedBox(height: Insets.sm),
            Text(
              'Sign in with your Apple Music subscription to see your saved '
              'music, playlists, and downloads.',
              textAlign: TextAlign.center,
              style: TextStyles.body.copyWith(color: palette.textSecondary),
            ),
            const SizedBox(height: Insets.lg),
            FilledButton(
              onPressed: onSignIn,
              style: FilledButton.styleFrom(
                backgroundColor: palette.accent,
                foregroundColor: Colors.white,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(Radii.small),
                ),
                padding: const EdgeInsets.symmetric(
                  horizontal: Insets.lg,
                  vertical: Insets.md,
                ),
              ),
              child: const Text('Sign In'),
            ),
          ],
        ),
      ),
    );
  }
}

