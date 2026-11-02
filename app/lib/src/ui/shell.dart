import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../state/sosumi_controller.dart';
import 'desktop_shell_parts.dart'
    show DesktopSidebar, DesktopTopBar, NowPlayingPanel;
import 'pages.dart';
import 'play_bar.dart';

/// The adaptive app shell: Apple Music-style mobile scaffold, desktop shell.
class Shell extends StatelessWidget {
  const Shell({super.key});

  @override
  Widget build(BuildContext context) {
    final controller = SosumiScope.of(context);

    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth < 760) {
          return _MobileShell(controller: controller);
        }
        return _DesktopShell(controller: controller);
      },
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
      AppSection.newReleases => NewPage(controller: controller),
      AppSection.radio => RadioPage(controller: controller),
      AppSection.library => LibraryPage(controller: controller),
      AppSection.search => SearchPage(controller: controller),
    };
  }
}

/// Mobile shell: iOS 26-style floating tab bar and play bar accessory.
class _MobileShell extends StatelessWidget {
  const _MobileShell({required this.controller});

  final SosumiController controller;

  LiquidGlassSettings _barGlassSettings(BuildContext context) {
    return LiquidGlassSettings(
      glassColor: const Color(0xAA1C1C1E),
      thickness: 30,
      blur: 2,
      lightAngle: GlassDefaults.lightAngle,
      lightIntensity: 0.20,
      ambientStrength: 0,
      refractiveIndex: 1.2,
      fresnelStrength: 0,
      saturation: 1.2,
      specularSharpness: GlassSpecularSharpness.medium,
    );
  }

  @override
  Widget build(BuildContext context) {
    return GlassScaffold(
      backgroundColor: Colors.black,
      statusBarStyle: GlassStatusBarStyle.light,
      settings: _barGlassSettings(context),
      body: SafeArea(
        top: true,
        bottom: false,
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
              child: SlideTransition(position: offset, child: child),
            );
          },
          child: KeyedSubtree(
            key: ValueKey(controller.section),
            child: _SectionView(controller: controller),
          ),
        ),
      ),
      bottomBar: GlassTabBar.bottom(
        key: const Key('mobile-tab-bar'),
        tabs: [
          for (final section in AppSection.values)
            GlassTab(
              label: section.label,
              icon: Icon(section.icon),
              activeIcon: Icon(section.icon),
            ),
        ],
        selectedIndex: controller.section.index,
        onTabSelected: (index) =>
            controller.selectSection(AppSection.values[index]),
        barHeight: 62,
        horizontalPadding: 14,
        verticalPadding: 10,
        spacing: 6,
        bottomAccessory: PlayBarPill(controller: controller),
        bottomAccessoryHeight: 56,
        selectedIconColor: controller.accent,
        unselectedIconColor: Colors.white.withValues(alpha: 0.70),
        selectedLabelColor: controller.accent,
        unselectedLabelColor: Colors.white.withValues(alpha: 0.70),
        labelFontSize: 10,
        iconSize: 26,
        settings: _barGlassSettings(context),
      ),
    );
  }
}

/// Desktop shell: sidebar, search bar, and now-playing panel.
class _DesktopShell extends StatelessWidget {
  const _DesktopShell({required this.controller});

  final SosumiController controller;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      body: LayoutBuilder(
        builder: (context, constraints) {
          final hasNowPlaying = constraints.maxWidth >= 1180;

          return Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              DesktopSidebar(controller: controller),
              Expanded(
                child: Column(
                  children: [
                    DesktopTopBar(controller: controller),
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
                    MiniPlayer(controller: controller),
                  ],
                ),
              ),
              if (hasNowPlaying) NowPlayingPanel(controller: controller),
            ],
          );
        },
      ),
    );
  }
}
