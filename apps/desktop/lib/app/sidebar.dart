import 'package:flutter/material.dart';

import '../theme/theme.dart';
import 'title_bar.dart';

/// The persistent left navigation.
///
/// It is one glass panel: the navigation items, then the user's playlists. A
/// collapsed state narrows it to icons, which is the behaviour a desktop user
/// expects from a sidebar.
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
        blur: 30,
        fill: palette.sidebarFill,
        showHighlight: false,
        padding: const EdgeInsets.symmetric(vertical: Insets.sm),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final item in AppSection.values)
              _SidebarItem(
                section: item,
                selected: item == section,
                collapsed: collapsed,
                onTap: () => onSelect(item),
              ),
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
    final color = selected ? palette.accent : palette.textSecondary;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: Insets.sm, vertical: 1),
      child: InteractiveGlass(
        onTap: onTap,
        selected: selected,
        radius: Radii.medium,
        padding: const EdgeInsets.symmetric(
          horizontal: Insets.md,
          vertical: Insets.sm + 2,
        ),
        child: Row(
          children: [
            Icon(section.icon, size: 18, color: color),
            if (!collapsed) ...[
              const SizedBox(width: Insets.md),
              Expanded(
                child: Text(
                  section.label,
                  style: TextStyles.control.copyWith(
                    color: selected ? palette.textPrimary : palette.textSecondary,
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
