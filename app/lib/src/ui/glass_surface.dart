import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

/// A translucent, blurred surface used for navigation and controls.
///
/// This uses the Flutter Liquid Glass renderer: full shader glass on Impeller
/// platforms, the lightweight shader on Windows/Linux, and the frosted fallback
/// where shaders are unavailable. It is a Flutter approximation, not a native
/// SwiftUI material, but it is shared across every target.
class GlassSurface extends StatelessWidget {
  const GlassSurface({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.radius = 24,
    this.frost = 14,
    this.enabled = true,
    this.highFidelity = true,
    this.tint,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;
  final double radius;
  final double frost;
  final bool enabled;
  final bool highFidelity;
  final Color? tint;

  @override
  Widget build(BuildContext context) {
    if (!enabled) {
      return _OpaqueSurface(padding: padding, radius: radius, child: child);
    }

    final brightness = Theme.of(context).brightness;
    final base = brightness == Brightness.dark
        ? GlassThemeSettings.ios27Dark
        : GlassThemeSettings.ios27Light;
    var settings = base
        .applyTo(const LiquidGlassSettings())
        .copyWith(frost: frost);
    if (tint != null) {
      settings = settings.copyWith(glassColor: tint);
    }

    // Premium glass is the full Impeller pipeline. Windows and Linux use the
    // lightweight shader path, which still renders the iOS-style rim/frost.
    final supportsPremium =
        !kIsWeb &&
        (defaultTargetPlatform == TargetPlatform.iOS ||
            defaultTargetPlatform == TargetPlatform.android ||
            defaultTargetPlatform == TargetPlatform.macOS);

    return AdaptiveGlass(
      shape: LiquidRoundedSuperellipse(borderRadius: radius),
      settings: settings,
      quality: supportsPremium && highFidelity
          ? GlassQuality.premium
          : GlassQuality.standard,
      useOwnLayer: true,
      child: Padding(padding: padding, child: child),
    );
  }
}

class _OpaqueSurface extends StatelessWidget {
  const _OpaqueSurface({
    required this.padding,
    required this.radius,
    required this.child,
  });

  final EdgeInsetsGeometry padding;
  final double radius;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(radius),
        color: const Color(0xFF15151D).withValues(alpha: 0.94),
        border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
      ),
      child: Padding(padding: padding, child: child),
    );
  }
}

/// A small status pill used in the now-playing surfaces.
class StatusPill extends StatelessWidget {
  const StatusPill({
    super.key,
    required this.label,
    this.icon = Icons.circle,
    this.color,
  });

  final String label;
  final IconData icon;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final effective = color ?? Theme.of(context).colorScheme.primary;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: effective.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: effective.withValues(alpha: 0.24)),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 12, color: effective),
            const SizedBox(width: 6),
            Text(
              label,
              style: Theme.of(context).textTheme.labelSmall
                  ?.copyWith(color: effective, fontWeight: FontWeight.w700),
            ),
          ],
        ),
      ),
    );
  }
}
