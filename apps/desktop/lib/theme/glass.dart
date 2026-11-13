import 'dart:ui';

import 'package:flutter/material.dart';

import 'palette.dart';
import 'tokens.dart';

/// A frosted, glass-like surface.
///
/// This is the one place the material is defined, so every panel in the app —
/// sidebar, player, sheets — is the same glass with the same edges. It composes
/// a blur, a translucent fill, a hairline border, and a specular highlight along
/// the top edge, which is what reads as a raised surface rather than a flat tint.
///
/// On a compositor without backdrop blur the blur still applies to what is
/// behind the widget within this window, so the result degrades to a within-app
/// blur rather than to a flat colour.
class GlassSurface extends StatelessWidget {
  const GlassSurface({
    super.key,
    required this.child,
    this.radius = Radii.panel,
    this.padding,
    this.fill,
    this.border,
    this.blur = 24,
    this.showHighlight = true,
    this.margin,
  });

  /// The content inside the glass.
  final Widget child;

  /// Outer corner radius.
  final double radius;

  /// Inner padding.
  final EdgeInsetsGeometry? padding;

  /// Overrides the base fill tint.
  final Color? fill;

  /// Overrides the hairline border.
  final Color? border;

  /// Blur strength. Higher reads as thicker glass.
  final double blur;

  /// Whether to draw the top-edge specular highlight.
  final bool showHighlight;

  /// Outer margin.
  final EdgeInsetsGeometry? margin;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final shape = BorderRadius.circular(radius);

    return Padding(
      padding: margin ?? EdgeInsets.zero,
      child: ClipRRect(
        borderRadius: shape,
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: blur, sigmaY: blur),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: fill ?? palette.glassFill,
              borderRadius: shape,
              border: Border.all(color: border ?? palette.glassBorder, width: 0.5),
            ),
            child: showHighlight
                ? Stack(
                    children: [
                      // A hairline of light along the top edge. Clipped to the
                      // top only, so it reads as a lit bevel rather than a frame.
                      Positioned(
                        top: 0,
                        left: radius * 0.5,
                        right: radius * 0.5,
                        child: Container(
                          height: 0.5,
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              colors: [
                                palette.glassHighlight.withValues(alpha: 0),
                                palette.glassHighlight,
                                palette.glassHighlight.withValues(alpha: 0),
                              ],
                            ),
                          ),
                        ),
                      ),
                      Padding(
                        padding: padding ?? EdgeInsets.zero,
                        child: child,
                      ),
                    ],
                  )
                : Padding(
                    padding: padding ?? EdgeInsets.zero,
                    child: child,
                  ),
          ),
        ),
      ),
    );
  }
}

/// A glass container that lifts slightly on hover.
///
/// Used for tappable glass: sidebar rows, cards, and controls. The lift is a
/// small scale and brightness change, which is enough to feel responsive
/// without drawing attention.
class InteractiveGlass extends StatefulWidget {
  const InteractiveGlass({
    super.key,
    required this.child,
    required this.onTap,
    this.radius = Radii.large,
    this.padding,
    this.selected = false,
    this.margin,
  });

  final Widget child;
  final VoidCallback? onTap;
  final double radius;
  final EdgeInsetsGeometry? padding;
  final bool selected;
  final EdgeInsetsGeometry? margin;

  @override
  State<InteractiveGlass> createState() => _InteractiveGlassState();
}

class _InteractiveGlassState extends State<InteractiveGlass> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return MouseRegion(
      cursor: widget.onTap == null ? MouseCursor.defer : SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        behavior: HitTestBehavior.opaque,
        child: AnimatedContainer(
          duration: Motion.fast,
          curve: Motion.enter,
          margin: widget.margin,
          decoration: BoxDecoration(
            color: widget.selected
                ? palette.accentSoft
                : _hovered
                    ? palette.glassFill
                    : Colors.transparent,
            borderRadius: BorderRadius.circular(widget.radius),
          ),
          child: Padding(
            padding: widget.padding ?? EdgeInsets.zero,
            child: widget.child,
          ),
        ),
      ),
    );
  }
}
