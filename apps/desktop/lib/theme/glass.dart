import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'palette.dart';
import 'tokens.dart';

/// A frosted surface for the functional layer.
///
/// Apple's model has two layers: **content** (text, images, lists, media) and
/// **functional** (sidebars, toolbars, and controls that float above content).
/// Liquid Glass belongs only on the functional layer, and it should be used
/// sparingly — glass on every card destroys the hierarchy the material exists
/// to create.
///
/// Real Liquid Glass differs from plain frosted glass in three ways, and this
/// widget does all three:
///
/// 1. **High transparency.** The fill is a light veil, not a panel. You should
///    be able to read the content behind it.
/// 2. **Saturation boost.** The blurred backdrop is over-saturated, which is
///    what makes colours behind the glass stay vivid instead of washing grey.
/// 3. **A specular edge.** A bright hairline where light would strike, fading
///    at the ends, so the surface reads as a lit bevel.
class GlassSurface extends StatelessWidget {
  const GlassSurface({
    super.key,
    required this.child,
    this.radius = Radii.panel,
    this.padding,
    this.fill,
    this.border,
    this.blur = 40,
    this.saturation = 1.8,
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

  /// Blur strength. Apple's regular material maps to roughly 20-40 px.
  final double blur;

  /// Backdrop saturation multiplier. 1.0 is neutral; Apple's vibrancy sits
  /// around 1.5-2.0.
  final double saturation;

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
          // Compose blur with a saturation matrix. Without the matrix the
          // backdrop desaturates as it blurs, which is what made the surface
          // read as grey frost rather than as glass.
          filter: ui.ImageFilter.compose(
            outer: ui.ImageFilter.blur(sigmaX: blur, sigmaY: blur),
            inner: ColorFilter.matrix(_saturationMatrix(saturation)),
          ),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: fill ?? palette.glassFill,
              borderRadius: shape,
              border: Border.all(color: border ?? palette.glassBorder, width: 0.5),
            ),
            child: showHighlight
                ? Stack(
                    children: [
                      // Light along the top edge, brightest at the centre and
                      // fading to nothing at the corners, so it reads as a
                      // specular highlight rather than as a border.
                      Positioned(
                        top: 0,
                        left: radius * 0.6,
                        right: radius * 0.6,
                        child: Container(
                          height: 1,
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              colors: [
                                palette.glassHighlight.withValues(alpha: 0),
                                palette.glassHighlight,
                                palette.glassHighlight,
                                palette.glassHighlight.withValues(alpha: 0),
                              ],
                              stops: const [0.0, 0.25, 0.75, 1.0],
                            ),
                          ),
                        ),
                      ),
                      Padding(padding: padding ?? EdgeInsets.zero, child: child),
                    ],
                  )
                : Padding(padding: padding ?? EdgeInsets.zero, child: child),
          ),
        ),
      ),
    );
  }
}

/// A luminance-preserving saturation matrix.
///
/// The coefficients are the standard Rec. 601 luma weights, which is what keeps
/// the perceived brightness of the backdrop unchanged as saturation rises.
List<double> _saturationMatrix(double s) => <double>[
      0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s, 0, 0, //
      0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s, 0, 0, //
      0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s, 0, 0, //
      0, 0, 0, 1, 0, //
    ];

/// A row that responds to hover and press.
///
/// The response is a fill change rather than a scale: scaling text under a
/// pointer makes it shimmer, and Apple's own controls brighten instead.
class InteractiveGlass extends StatefulWidget {
  const InteractiveGlass({
    super.key,
    required this.child,
    required this.onTap,
    this.radius = Radii.row,
    this.padding,
    this.selected = false,
    this.margin,
    this.tooltip,
  });

  final Widget child;
  final VoidCallback? onTap;
  final double radius;
  final EdgeInsetsGeometry? padding;
  final bool selected;
  final EdgeInsetsGeometry? margin;
  final String? tooltip;

  @override
  State<InteractiveGlass> createState() => _InteractiveGlassState();
}

class _InteractiveGlassState extends State<InteractiveGlass> {
  bool _hovered = false;
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    final decoration = BoxDecoration(
      color: widget.selected
          ? palette.selection
          : _pressed
              ? palette.fill
              : _hovered
                  ? palette.fill.withValues(alpha: palette.fill.a * 0.6)
                  : Colors.transparent,
      borderRadius: BorderRadius.circular(widget.radius),
    );

    Widget row = AnimatedContainer(
      duration: Motion.fast,
      curve: Motion.enter,
      margin: widget.margin,
      decoration: decoration,
      child: Padding(padding: widget.padding ?? EdgeInsets.zero, child: widget.child),
    );

    row = MouseRegion(
      cursor: widget.onTap == null ? MouseCursor.defer : SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() {
        _hovered = false;
        _pressed = false;
      }),
      child: GestureDetector(
        onTapDown: widget.onTap == null ? null : (_) => setState(() => _pressed = true),
        onTapUp: widget.onTap == null ? null : (_) => setState(() => _pressed = false),
        onTapCancel: widget.onTap == null ? null : () => setState(() => _pressed = false),
        onTap: widget.onTap,
        behavior: HitTestBehavior.opaque,
        child: row,
      ),
    );

    final tooltip = widget.tooltip;
    if (tooltip != null) {
      row = Tooltip(message: tooltip, child: row);
    }
    return row;
  }
}
