import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'aurora_background.dart';
import 'feedback.dart';
import 'glass_surface.dart';
import 'now_playing.dart';
import 'pages.dart';
import 'settings_sheet.dart';

/// The adaptive app shell: sidebar on wide layouts, dock on compact layouts.
class Shell extends StatelessWidget {
  const Shell({super.key});

  @override
  Widget build(BuildContext context) {
    final controller = SosumiScope.of(context);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: AuroraBackground(
        accent: controller.accent,
        animate: controller.motionEnabled,
        child: SafeArea(
          child: LayoutBuilder(
            builder: (context, constraints) {
              final width = constraints.maxWidth;
              final hasSidebar = width >= 760;
              final hasNowPlaying = width >= 1180;

              return Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (hasSidebar) _Sidebar(controller: controller),
                  Expanded(
                    child: Column(
                      children: [
                        _TopBar(
                          controller: controller,
                          compact: !hasSidebar,
                          showSearch: width >= 920,
                        ),
                        Expanded(
                          child: AnimatedSwitcher(
                            duration: controller.motionEnabled
                                ? const Duration(milliseconds: 260)
                                : Duration.zero,
                            switchInCurve: Curves.easeOutCubic,
                            switchOutCurve: Curves.easeInCubic,
                            transitionBuilder: (child, animation) {
                              final offset = Tween<Offset>(
                                begin: const Offset(0, 0.012),
                                end: Offset.zero,
                              ).animate(animation);
                              return FadeTransition(
                                opacity: animation,
                                child: SlideTransition(
                                  position: offset,
                                  child: child,
                                ),
                              );
                            },
                            child: KeyedSubtree(
                              key: ValueKey(controller.section),
                              child: _SectionView(controller: controller),
                            ),
                          ),
                        ),
                        if (!hasNowPlaying) MiniPlayer(controller: controller),
                        if (!hasSidebar) _CompactDock(controller: controller),
                      ],
                    ),
                  ),
                  if (hasNowPlaying) NowPlayingPanel(controller: controller),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}

class _SectionView extends StatelessWidget {
  const _SectionView({required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return switch (controller.section) {
      AppSection.home => HomePage(controller: controller),
      AppSection.browse => BrowsePage(controller: controller),
      AppSection.radio => RadioPage(controller: controller),
      AppSection.library => LibraryPage(controller: controller),
    };
  }
}

class _TopBar extends StatelessWidget {
  const _TopBar({
    required this.controller,
    required this.compact,
    required this.showSearch,
  });

  final SosumiController controller;
  final bool compact;
  final bool showSearch;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.fromLTRB(24, 18, 20, 12),
      child: Row(
        children: [
          if (compact) ...[
            _LogoMark(size: 32, accent: controller.accent),
            const SizedBox(width: 12),
          ],
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  controller.section.label,
                  style: theme.textTheme.titleLarge,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                if (!compact)
                  Text(
                    'Apple Music, with better manners.',
                    style: theme.textTheme.bodySmall,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
              ],
            ),
          ),
          if (showSearch) ...[
            SizedBox(
              width: 300,
              height: 46,
              child: TextField(
                readOnly: true,
                onTap: () => showScaffoldNote(context, 'Search'),
                decoration: const InputDecoration(
                  hintText: 'Search Apple Music',
                  prefixIcon: Icon(Icons.search_rounded),
                ),
              ),
            ),
            const SizedBox(width: 10),
          ],
          IconButton(
            tooltip: 'Settings',
            onPressed: () => showSettingsSheet(context),
            icon: const Icon(Icons.tune_rounded),
          ),
        ],
      ),
    );
  }
}

class _Sidebar extends StatelessWidget {
  const _Sidebar({required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      width: 236,
      decoration: BoxDecoration(
        border: Border(
          right: BorderSide(color: Colors.white.withValues(alpha: 0.07)),
        ),
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [Colors.white.withValues(alpha: 0.035), Colors.transparent],
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 20, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                _LogoMark(size: 36, accent: controller.accent),
                const SizedBox(width: 12),
                Text('sosumi', style: theme.textTheme.titleLarge),
              ],
            ),
            const SizedBox(height: 28),
            for (final section in AppSection.values) ...[
              _SidebarNavButton(
                section: section,
                selected: controller.section == section,
                accent: controller.accent,
                onTap: () => controller.selectSection(section),
              ),
              const SizedBox(height: 6),
            ],
            const Spacer(),
            GlassSurface(
              enabled: controller.glassEnabled,
              radius: 18,
              padding: const EdgeInsets.all(10),
              child: InkWell(
                borderRadius: BorderRadius.circular(12),
                onTap: () => showScaffoldNote(context, 'Account'),
                child: Row(
                  children: [
                    CircleAvatar(
                      radius: 16,
                      backgroundColor: Colors.white.withValues(alpha: 0.08),
                      child: Icon(
                        Icons.person_rounded,
                        size: 18,
                        color: Colors.white.withValues(alpha: 0.7),
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            'Not signed in',
                            style: theme.textTheme.labelLarge,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            'Connect Apple Music',
                            style: theme.textTheme.bodySmall,
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
          ],
        ),
      ),
    );
  }
}

class _SidebarNavButton extends StatelessWidget {
  const _SidebarNavButton({
    required this.section,
    required this.selected,
    required this.accent,
    required this.onTap,
  });

  final AppSection section;
  final bool selected;
  final Color accent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(14),
        onTap: onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 180),
          curve: Curves.easeOutCubic,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 11),
          decoration: BoxDecoration(
            color: selected
                ? accent.withValues(alpha: 0.16)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(14),
            border: Border.all(
              color: selected
                  ? accent.withValues(alpha: 0.22)
                  : Colors.transparent,
            ),
          ),
          child: Row(
            children: [
              Icon(
                section.icon,
                size: 20,
                color: selected ? accent : Colors.white.withValues(alpha: 0.55),
              ),
              const SizedBox(width: 12),
              Text(
                section.label,
                style: theme.textTheme.titleMedium?.copyWith(
                  color: selected
                      ? Colors.white
                      : Colors.white.withValues(alpha: 0.66),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CompactDock extends StatelessWidget {
  const _CompactDock({required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
      child: GlassSurface(
        key: const Key('compact-dock'),
        enabled: controller.glassEnabled,
        radius: 24,
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
        child: NavigationBar(
          height: 62,
          selectedIndex: controller.section.index,
          labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
          onDestinationSelected: (index) =>
              controller.selectSection(AppSection.values[index]),
          destinations: [
            for (final section in AppSection.values)
              NavigationDestination(
                icon: Icon(section.icon),
                selectedIcon: Icon(section.icon),
                label: section.label,
              ),
          ],
        ),
      ),
    );
  }
}

class _LogoMark extends StatelessWidget {
  const _LogoMark({required this.size, required this.accent});

  final double size;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(size * 0.32),
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [accent, accent.withValues(alpha: 0.42)],
        ),
        boxShadow: [
          BoxShadow(
            color: accent.withValues(alpha: 0.26),
            blurRadius: 18,
            offset: const Offset(0, 6),
          ),
        ],
      ),
      child: Icon(
        Icons.graphic_eq_rounded,
        size: size * 0.56,
        color: Colors.black.withValues(alpha: 0.78),
      ),
    );
  }
}
