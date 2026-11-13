import 'package:flutter/material.dart';

import '../theme/theme.dart';

/// A navigation destination in the sidebar.
enum AppSection {
  listen('Listen', Icons.play_circle_outline_rounded),
  browse('Browse', Icons.grid_view_rounded),
  library('Library', Icons.library_music_outlined),
  search('Search', Icons.search_rounded);

  const AppSection(this.label, this.icon);
  final String label;
  final IconData icon;
}

/// The window's title bar.
///
/// It is a drag region, and it hosts the sidebar toggle and window actions. The
/// window is frameless, so this bar is also the top of the glass canvas.
class TitleBar extends StatelessWidget {
  const TitleBar({
    super.key,
    required this.sidebarOpen,
    required this.onToggleSidebar,
  });

  final bool sidebarOpen;
  final VoidCallback onToggleSidebar;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return SizedBox(
      height: Sizes.titleBarHeight,
      child: Row(
        children: [
          const SizedBox(width: Insets.md),
          _BarButton(
            icon: sidebarOpen
                ? Icons.splitscreen_rounded
                : Icons.splitscreen_outlined,
            tooltip: sidebarOpen ? 'Hide sidebar' : 'Show sidebar',
            onTap: onToggleSidebar,
          ),
          // The rest of the bar drags the window.
          Expanded(
            child: GestureDetector(
              behavior: HitTestBehavior.translucent,
              onPanStart: (_) => _startDrag(),
              child: Center(
                child: Text(
                  'Sosumi',
                  style: TextStyles.caption.copyWith(color: palette.textSecondary),
                ),
              ),
            ),
          ),
          const SizedBox(width: Insets.md),
        ],
      ),
    );
  }

  void _startDrag() {
    // Window dragging is wired through the platform channel once the Linux
    // embedder adds it; a no-op keeps this bar inert rather than broken.
  }
}

/// A small icon button styled for the glass chrome.
class _BarButton extends StatelessWidget {
  const _BarButton({required this.icon, required this.tooltip, required this.onTap});

  final IconData icon;
  final String tooltip;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InteractiveGlass(
        onTap: onTap,
        radius: Radii.small,
        padding: const EdgeInsets.all(Insets.sm),
        child: Icon(icon, size: 16, color: context.palette.textSecondary),
      ),
    );
  }
}
