import 'package:flutter/material.dart';

import '../features/account/account_row.dart';
import '../theme/theme.dart';

/// A top-level destination.
///
/// The set and order mirror Apple Music's own sidebar, which is the fastest way
/// for someone arriving from Apple's app to feel at home: Search first (it is
/// the most-used destination), then Home, New, and Radio.
enum AppSection {
  search('Search', Icons.search_rounded),
  home('Home', Icons.home_outlined),
  newReleases('New', Icons.grid_view_rounded),
  radio('Radio', Icons.radio_outlined),
  recentlyAdded('Recently Added', Icons.schedule_outlined),
  artists('Artists', Icons.mic_none_outlined),
  albums('Albums', Icons.album_outlined),
  songs('Songs', Icons.music_note_outlined),
  playlists('Playlists', Icons.queue_music_outlined),
  settings('Settings', Icons.settings_outlined);

  const AppSection(this.label, this.icon);
  final String label;
  final IconData icon;

  /// The browse group, above the Library heading.
  static const browse = [search, home, newReleases, radio];

  /// The Library group, matching Apple Music's own list.
  static const personal = [recentlyAdded, artists, albums, songs, playlists];
}

/// The persistent left navigation.
///
/// One glass panel, as Apple specifies: the sidebar *is* the functional layer, so
/// this is one of the few places the material belongs. The account control is
/// pinned to the foot, which is where the reference app puts it and where a
/// pointer naturally ends a vertical scan.
class Sidebar extends StatelessWidget {
  const Sidebar({
    super.key,
    required this.section,
    required this.onSelect,
    required this.collapsed,
  });

  final AppSection section;
  final ValueChanged<AppSection> onSelect;
  final bool collapsed;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return AnimatedContainer(
      duration: Motion.normal,
      curve: Motion.enter,
      width: collapsed ? Sizes.sidebarCollapsedWidth : Sizes.sidebarWidth,
      child: GlassSurface(
        radius: 0,
        blur: 44,
        saturation: 1.9,
        // Deliberately barely-there: at rest the sidebar is almost all backdrop,
        // with only enough veil to hold text legible over any wallpaper.
        fill: palette.sidebarFill,
        border: Colors.transparent,
        showHighlight: false,
        padding: const EdgeInsets.symmetric(vertical: Insets.sm, horizontal: Insets.sm),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final item in AppSection.browse)
              _SidebarItem(
                section: item,
                selected: item == section,
                collapsed: collapsed,
                onTap: () => onSelect(item),
              ),
            const SizedBox(height: Insets.md),
            if (!collapsed)
              Padding(
                padding: const EdgeInsets.only(left: Insets.md, bottom: Insets.xs),
                child: Text(
                  'Library',
                  style: TextStyles.subheadline.copyWith(color: palette.textSecondary),
                ),
              ),
            for (final item in AppSection.personal)
              _SidebarItem(
                section: item,
                selected: item == section,
                collapsed: collapsed,
                onTap: () => onSelect(item),
              ),
            const Spacer(),
            // The HIG warns against critical actions at the foot of a sidebar
            // because a relocated window can hide them. The account menu is not
            // critical: every command in it is reachable elsewhere.
            AccountRow(collapsed: collapsed),
          ],
        ),
      ),
    );
  }
}

class _SidebarItem extends StatelessWidget {
  const _SidebarItem({
    required this.section,
    required this.selected,
    required this.collapsed,
    required this.onTap,
  });

  final AppSection section;
  final bool selected;
  final bool collapsed;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    // Apple tints the selected sidebar icon with the accent and leaves the rest
    // at primary. That is the one place the accent belongs in a sidebar.
    final iconColor = selected ? palette.accent : palette.textPrimary;
    final labelColor = selected ? palette.accent : palette.textPrimary;

    return InteractiveGlass(
      onTap: onTap,
      selected: selected,
      radius: Radii.row,
      tooltip: collapsed ? section.label : null,
      padding: const EdgeInsets.symmetric(horizontal: Insets.sm),
      margin: const EdgeInsets.symmetric(vertical: 1),
      child: SizedBox(
        height: Sizes.sidebarRowHeight,
        child: Row(
          children: [
            Icon(section.icon, size: 16, color: iconColor),
            if (!collapsed) ...[
              const SizedBox(width: Insets.sm),
              Expanded(
                child: Text(
                  section.label,
                  style: TextStyles.body.copyWith(color: labelColor),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
