import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/engine.dart';
import '../core/player.dart';
import '../features/library/library_page.dart';
import '../features/player/player_bar.dart';
import '../features/search/search_page.dart';
import '../features/settings/settings_page.dart';
import '../theme/theme.dart';
import 'sidebar.dart';
import 'title_bar.dart';

/// The main window.
///
/// Layout follows Apple's macOS shape: a full-height glass sidebar on the
/// leading edge, the content pane beside it, and the transport floating over the
/// content rather than docked to its edge.
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

    return DefaultTextStyle(
      // The ancestor every Text inherits. Without it, Flutter paints text with
      // its `fallback` style, which is a yellow double underline — the
      // framework's signal that a DefaultTextStyle ancestor is missing.
      style: TextStyles.body.copyWith(color: palette.textPrimary),
      child: Material(
        // Text fields and other ink-bearing widgets require a Material ancestor.
        // `transparency` provides one without painting a background, which
        // matters because the app draws its own canvas.
        type: MaterialType.transparency,
        child: DecoratedBox(
          decoration: _canvasBackground(palette),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Sidebar(
                section: _section,
                collapsed: !_sidebarOpen,
                onSelect: (section) => setState(() => _section = section),
              ),
              // A hairline between the sidebar and the content, which is what
              // macOS uses in place of a shadow.
              VerticalDivider(width: 0.5, thickness: 0.5, color: palette.separator),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Toolbar(
                      title: _section.label,
                      sidebarOpen: _sidebarOpen,
                      onToggleSidebar: () => setState(() => _sidebarOpen = !_sidebarOpen),
                    ),
                    Expanded(
                      child: Stack(
                        children: [
                          Positioned.fill(
                            // A page transition rather than an instant swap, so
                            // the change of context is legible.
                            child: AnimatedSwitcher(
                              duration: Motion.normal,
                              switchInCurve: Motion.enter,
                              switchOutCurve: Motion.exit,
                              child: KeyedSubtree(
                                key: ValueKey(_section),
                                child: _SectionView(
                                  section: _section,
                                  onOpenSettings: () =>
                                      setState(() => _section = AppSection.settings),
                                ),
                              ),
                            ),
                          ),
                          // The player floats over the content, inset from the
                          // edges so the material has content to refract.
                          const Positioned(left: 0, right: 0, bottom: 0, child: PlayerBar()),
                        ],
                      ),
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

/// The window's backdrop.
///
/// Apple's rule is that content extends beneath the sidebar and toolbar so the
/// glass has something to refract; glass over a flat fill reads as a grey panel
/// instead of a material. With no artwork loaded yet, a very subtle accent wash
/// gives the material a gradient to pick up without competing with the album
/// artwork that will sit on top of it.
BoxDecoration _canvasBackground(SosumiPalette palette) {
  final tint = Color.lerp(palette.canvas, palette.accent, 0.10) ?? palette.canvas;
  return BoxDecoration(
    gradient: LinearGradient(
      begin: Alignment.topRight,
      end: Alignment.bottomLeft,
      colors: [palette.canvas, palette.canvas, tint],
      stops: const [0.0, 0.55, 1.0],
    ),
  );
}

/// The page for a section.
///
/// Each page supplies its own content only: the section name already appears in
/// the toolbar, so repeating it as a heading inside the page would say the same
/// thing twice.
class _SectionView extends StatelessWidget {
  const _SectionView({required this.section, required this.onOpenSettings});

  final AppSection section;
  final VoidCallback onOpenSettings;

  @override
  Widget build(BuildContext context) {
    return switch (section) {
      AppSection.search => const SearchPage(),
      AppSection.settings => const SettingsPage(),
      AppSection.recentlyAdded ||
      AppSection.artists ||
      AppSection.albums ||
      AppSection.songs ||
      AppSection.playlists =>
        LibraryPage(section: section, onSignIn: onOpenSettings),
      AppSection.home => const _Placeholder(
          icon: Icons.home_outlined,
          title: 'Home',
          detail: 'Your personalised mixes and recently played will appear here.',
        ),
      AppSection.newReleases => const _Placeholder(
          icon: Icons.grid_view_rounded,
          title: 'New',
          detail: 'New releases, charts, and editor picks will appear here.',
        ),
      AppSection.radio => const _Placeholder(
          icon: Icons.radio_outlined,
          title: 'Radio',
          detail: 'Stations and live shows will appear here.',
        ),
    };
  }
}

/// An empty state that invites the next action.
class _Placeholder extends StatelessWidget {
  const _Placeholder({required this.icon, required this.title, required this.detail});

  final IconData icon;
  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 320),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 36, color: palette.textTertiary),
            const SizedBox(height: Insets.lg),
            Text(title, style: TextStyles.title3.copyWith(color: palette.textPrimary)),
            const SizedBox(height: Insets.sm),
            Text(
              detail,
              textAlign: TextAlign.center,
              style: TextStyles.body.copyWith(color: palette.textSecondary),
            ),
          ],
        ),
      ),
    );
  }
}

/// The root widget.
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
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final engine = ref.read(engineProvider).requireValue;
      // The bridge starts audio when it creates the engine; this only wires the
      // event stream into the controller.
      ref.read(playerProvider.notifier).attach(engine.watchEvents());
    });
  }

  @override
  Widget build(BuildContext context) => const AppShell();
}

class _Boot extends StatelessWidget {
  const _Boot();

  @override
  Widget build(BuildContext context) {
    return const ColoredBox(
      color: Color(0xFF1C1C1E),
      child: Center(
        child: SizedBox(
          width: 20,
          height: 20,
          child: CircularProgressIndicator(strokeWidth: 2, color: Color(0xFF8E8E93)),
        ),
      ),
    );
  }
}

class _BootFailure extends StatelessWidget {
  const _BootFailure({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    const palette = SosumiPalette.dark;
    return ColoredBox(
      color: palette.canvas,
      child: DefaultTextStyle(
        style: TextStyles.body.copyWith(color: palette.textPrimary),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 380),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.warning_amber_rounded, size: 28, color: Color(0xFFFF453A)),
                const SizedBox(height: Insets.lg),
                Text('Sosumi could not start',
                    style: TextStyles.title3.copyWith(color: palette.textPrimary)),
                const SizedBox(height: Insets.sm),
                Text(
                  message,
                  textAlign: TextAlign.center,
                  style: TextStyles.callout.copyWith(color: palette.textSecondary),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
