import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../theme/theme.dart';

/// The window's toolbar.
///
/// Scope, per Apple's toolbar guidance and the reference app: the current view's
/// title, then **actions on that view**. It deliberately does not carry account
/// state, a queue button, or a settings shortcut — the account menu owns account
/// commands, the player bar owns the queue, and Settings is a sidebar
/// destination. Repeating them here duplicated every one of them.
///
/// The window is undecorated, so this bar is also the window's drag handle. That
/// is why it is full width and why its empty space accepts a drag.
class Toolbar extends StatelessWidget {
  const Toolbar({
    super.key,
    required this.title,
    required this.sidebarOpen,
    required this.onToggleSidebar,
  });

  final String title;
  final bool sidebarOpen;
  final VoidCallback onToggleSidebar;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return SizedBox(
      height: 48,
      child: Row(
        children: [
          const SizedBox(width: Insets.md),
          _ToolbarButton(
            icon: Icons.vertical_split_outlined,
            tooltip: sidebarOpen ? 'Hide Sidebar' : 'Show Sidebar',
            onTap: onToggleSidebar,
          ),
          const SizedBox(width: Insets.md),
          Text(title, style: TextStyles.title3.copyWith(color: palette.textPrimary)),
          const Spacer(),
          const SizedBox(width: Insets.md),
        ],
      ),
    );
  }
}

/// `_ToolbarButton` is private to this file.
class _ToolbarButton extends ConsumerWidget {
  const _ToolbarButton({
    required this.icon,
    required this.tooltip,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    return InteractiveGlass(
      onTap: onTap,
      radius: Radii.small,
      tooltip: tooltip,
      padding: const EdgeInsets.all(6),
      child: Icon(
        icon,
        size: 16,
        color: onTap == null ? palette.textTertiary : palette.textSecondary,
      ),
    );
  }
}
