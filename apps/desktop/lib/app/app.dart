import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/engine.dart';
import '../core/player.dart';
import '../features/player/player_bar.dart';
import '../features/search/search_page.dart';
import '../features/settings/settings_page.dart';
import '../theme/theme.dart';
import 'sidebar.dart';
import 'title_bar.dart';

/// The main window.
///
/// The layout is a fixed sidebar, the selected section, and a floating player.
/// The canvas behind everything is where the wallpaper-tinted backdrop goes;
/// today it is the palette's canvas, which the glass panels blur.
class AppShell extends ConsumerStatefulWidget {
  const AppShell({super.key});

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  AppSection _section = AppSection.search;
  bool _sidebarOpen = true;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return DecoratedBox(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            palette.canvas,
            Color.lerp(palette.canvas, palette.accent, 0.06) ?? palette.canvas,
            palette.canvas,
          ],
        ),
      ),
      child: Column(
        children: [
          TitleBar(
            sidebarOpen: _sidebarOpen,
            onToggleSidebar: () => setState(() => _sidebarOpen = !_sidebarOpen),
          ),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Sidebar(
                  section: _section,
                  collapsed: !_sidebarOpen,
                  onSelect: (section) => setState(() => _section = section),
                ),
                Expanded(
                  child: Stack(
                    children: [
                      Positioned.fill(
                        child: _SectionView(section: _section),
                      ),
                      // The player floats above the content, inset from the
                      // sidebar, so the glass reads over the page behind it.
                      Positioned(
                        left: 0,
                        right: 0,
                        bottom: 0,
                        child: const IgnorePointer(
                          ignoring: false,
                          child: PlayerBar(),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// The visible page for a section.
class _SectionView extends StatelessWidget {
  const _SectionView({required this.section});

  final AppSection section;

  @override
  Widget build(BuildContext context) {
    return switch (section) {
      AppSection.search => const SearchPage(),
      AppSection.listen => const _ComingSoon(
          icon: Icons.play_circle_outline_rounded,
          title: 'Listen',
          detail: 'Your personalised mixes and recently played will appear here.',
        ),
      AppSection.browse => const SettingsPage(),
      AppSection.library => const _ComingSoon(
          icon: Icons.library_music_outlined,
          title: 'Library',
          detail: 'Sign in to see your saved songs, albums, and playlists.',
        ),
    };
  }
}

class _ComingSoon extends StatelessWidget {
  const _ComingSoon({
    required this.icon,
    required this.title,
    required this.detail,
  });

  final IconData icon;
  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 40, color: palette.textSecondary.withValues(alpha: 0.5)),
          const SizedBox(height: Insets.lg),
          Text(title, style: TextStyles.title.copyWith(color: palette.textPrimary)),
          const SizedBox(height: Insets.sm),
          Text(detail, style: TextStyles.body.copyWith(color: palette.textSecondary)),
        ],
      ),
    );
  }
}

/// The root widget.
///
/// It waits for the engine before showing the shell, because every screen reads
/// from it. A failure here is fatal and shown rather than swallowed, so a broken
/// bridge is never mistaken for an empty library.
class SosumiApp extends ConsumerWidget {
  const SosumiApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final engine = ref.watch(engineProvider);

    return MaterialApp(
      title: 'Sosumi',
      debugShowCheckedModeBanner: false,
      theme: buildTheme(Brightness.dark),
      home: engine.when(
        loading: () => const _Boot(),
        error: (error, _) => _BootFailure(message: '$error'),
        data: (_) => const _EngineReady(),
      ),
    );
  }
}

/// Bridges the engine's event stream into the player controller once ready.
class _EngineReady extends ConsumerStatefulWidget {
  const _EngineReady();

  @override
  ConsumerState<_EngineReady> createState() => _EngineReadyState();
}

class _EngineReadyState extends ConsumerState<_EngineReady> {
  @override
  void initState() {
    super.initState();
    // Attach after the first frame so the engine and the controller both exist.
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final engine = ref.read(engineProvider).requireValue;
      final player = ref.read(playerProvider.notifier);
      player.attach(engine.watchEvents());
      await player.startAudio();
    });
  }

  @override
  Widget build(BuildContext context) => const AppShell();
}

class _Boot extends StatelessWidget {
  const _Boot();

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: SosumiPalette.dark.canvas,
      child: const Center(child: CircularProgressIndicator(strokeWidth: 2)),
    );
  }
}

class _BootFailure extends StatelessWidget {
  const _BootFailure({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = SosumiPalette.dark;
    return ColoredBox(
      color: palette.canvas,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(Insets.xl),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.warning_amber_rounded,
                size: 32,
                color: palette.accent,
              ),
              const SizedBox(height: Insets.lg),
              Text(
                'Sosumi could not start',
                style: TextStyles.title.copyWith(color: palette.textPrimary),
              ),
              const SizedBox(height: Insets.sm),
              Text(
                message,
                textAlign: TextAlign.center,
                style: TextStyles.body.copyWith(color: palette.textSecondary),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
