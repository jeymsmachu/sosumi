import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'artwork.dart';
import 'now_playing_page.dart';
import 'settings_page.dart';

/// Desktop top bar: search and account on the right.
class DesktopTopBar extends StatelessWidget {
  const DesktopTopBar({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 12, 16, 12),
      child: Row(
        children: [
          const Spacer(),
          SizedBox(
            width: 320,
            height: 44,
            child: TextField(
              readOnly: true,
              onTap: () => controller.selectSection(AppSection.search),
              decoration: const InputDecoration(
                hintText: 'Search Apple Music',
                prefixIcon: Icon(Icons.search_rounded),
              ),
            ),
          ),
          const SizedBox(width: 10),
          CircleAvatar(
            radius: 18,
            backgroundColor: Colors.white.withValues(alpha: 0.09),
            child: Text(
              'S',
              style: Theme.of(context).textTheme.labelLarge
                  ?.copyWith(color: Colors.white.withValues(alpha: 0.88)),
            ),
          ),
          const SizedBox(width: 4),
          IconButton(
            tooltip: 'Settings',
            onPressed: () => showSettingsPage(context),
            icon: const Icon(Icons.more_horiz_rounded),
          ),
        ],
      ),
    );
  }
}

/// Desktop sidebar: black, compact, Apple Music-style.
class DesktopSidebar extends StatelessWidget {
  const DesktopSidebar({super.key, required this.controller});

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
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 18, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Container(
                  width: 34,
                  height: 34,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(11),
                    gradient: LinearGradient(
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                      colors: [
                        controller.accent,
                        controller.accent.withValues(alpha: 0.5),
                      ],
                    ),
                  ),
                  child: const Icon(
                    Icons.music_note_rounded,
                    size: 18,
                    color: Colors.white,
                  ),
                ),
                const SizedBox(width: 12),
                Text('sosumi', style: theme.textTheme.titleLarge),
              ],
            ),
            const SizedBox(height: 24),
            for (final section in AppSection.values) ...[
              _SidebarNavButton(
                section: section,
                selected: controller.section == section,
                accent: controller.accent,
                onTap: () => controller.selectSection(section),
              ),
              const SizedBox(height: 4),
            ],
            const Spacer(),
            Material(
              color: Colors.white.withValues(alpha: 0.05),
              borderRadius: BorderRadius.circular(18),
              child: InkWell(
                borderRadius: BorderRadius.circular(18),
                onTap: () => showSettingsPage(context),
                child: Padding(
                  padding: const EdgeInsets.all(10),
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
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          decoration: BoxDecoration(
            color: selected
                ? accent.withValues(alpha: 0.16)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(14),
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

/// Desktop now-playing panel with a full-page player handoff.
class NowPlayingPanel extends StatelessWidget {
  const NowPlayingPanel({super.key, required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return SizedBox(
      width: 328,
      child: Material(
        color: const Color(0xFF101010),
        borderRadius: BorderRadius.circular(24),
        child: InkWell(
          borderRadius: BorderRadius.circular(24),
          onTap: () => showNowPlayingPage(context),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'NOW PLAYING',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: Colors.white.withValues(alpha: 0.48),
                    letterSpacing: 1.6,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 14),
                const Expanded(child: AlbumArt(seed: 7, radius: 18)),
                const SizedBox(height: 18),
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
                const SizedBox(height: 14),
                ClipRRect(
                  borderRadius: BorderRadius.circular(999),
                  child: LinearProgressIndicator(
                    value: 0.34,
                    minHeight: 4,
                    backgroundColor: Colors.white.withValues(alpha: 0.10),
                    color: controller.accent,
                  ),
                ),
                const SizedBox(height: 16),
                FilledButton.tonalIcon(
                  onPressed: () => showNowPlayingPage(context),
                  icon: const Icon(Icons.play_arrow_rounded),
                  label: const Text('Open player'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
