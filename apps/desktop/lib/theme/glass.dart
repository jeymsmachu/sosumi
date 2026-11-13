import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'motion.dart';
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

    // The accessibility states, from the Liquid Glass checklist: "Reduce
    // Transparency gets an opaque fallback, Increase Contrast gets stronger
    // fills and borders." Flutter exposes high contrast but not reduce
    // transparency, so both are served by the same fallback, which is the safe
    // direction: an opaque surface is legible everywhere a translucent one is,
    // and the reverse is not true.
    final highContrast = MediaQuery.maybeHighContrastOf(context) ?? false;
    if (highContrast) {
      return Padding(
        padding: margin ?? EdgeInsets.zero,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: _opaque(palette, fill),
            borderRadius: shape,
            border: Border.all(color: palette.textTertiary, width: 1),
          ),
          child: Padding(padding: padding ?? EdgeInsets.zero, child: child),
        ),
      );
    }

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

  /// A translucent tint resolved to something opaque, for the accessibility
  /// fallback.
  ///
  /// Uses the canvas colour the tint was composed against, so the fallback keeps
  /// the same value rather than inventing a new one.
  Color _opaque(SosumiPalette palette, Color? tint) =>
      Color.alphaBlend(tint ?? palette.glassFill, palette.canvas);
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
///
/// The change animates on a short spring rather than a linear curve, so a quick
/// pass of the pointer and a deliberate press both feel like the same material
/// responding. `motion.md`: "Aim for brevity and precision in feedback
/// animations."
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
    this.semanticLabel,
  });

  final Widget child;
  final VoidCallback? onTap;
  final double radius;
  final EdgeInsetsGeometry? padding;
  final bool selected;
  final EdgeInsetsGeometry? margin;
  final String? tooltip;

  /// A label for assistive technology, when the child is an icon alone.
  ///
  /// `accessibility.md`: "Convey information with more than color alone" and the
  /// requirement that every icon-only control be labelled. A tooltip is not a
  /// substitute, because it is not announced.
  final String? semanticLabel;

  @override
  State<InteractiveGlass> createState() => _InteractiveGlassState();
}

class _InteractiveGlassState extends State<InteractiveGlass> {
  bool _hovered = false;
  bool _pressed = false;
  bool _focused = false;
  final _focus = FocusNode(debugLabel: 'InteractiveGlass');

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final interactive = widget.onTap != null;

    final decoration = BoxDecoration(
      color: widget.selected
          ? palette.selection
          : _pressed
              ? palette.fill
              : _hovered
                  ? palette.fill.withValues(alpha: palette.fill.a * 0.6)
                  : Colors.transparent,
      borderRadius: BorderRadius.circular(widget.radius),
      // A focus ring rather than a highlight, because these are controls rather
      // than list rows. `focus-and-selection.md`: "use a focus ring for a text or
      // search field, but use a highlight in a list or collection". The accent
      // is the platform's own focus colour, so it matches the system.
      border: _focused ? Border.all(color: palette.accent, width: 2) : null,
    );

    Widget row = AnimatedContainer(
      duration: context.motionDuration(Motion.fast),
      curve: context.motionCurve(Motion.interactive),
      margin: widget.margin,
      decoration: decoration,
      child: Padding(padding: widget.padding ?? EdgeInsets.zero, child: widget.child),
    );

    row = MouseRegion(
      cursor: interactive ? SystemMouseCursors.click : MouseCursor.defer,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() {
        _hovered = false;
        _pressed = false;
      }),
      child: GestureDetector(
        onTapDown: interactive ? (_) => setState(() => _pressed = true) : null,
        onTapUp: interactive ? (_) => setState(() => _pressed = false) : null,
        onTapCancel: interactive ? () => setState(() => _pressed = false) : null,
        onTap: widget.onTap,
        behavior: HitTestBehavior.opaque,
        child: row,
      ),
    );

    if (interactive) {
      // Keyboard-reachable, and activating on Enter or Space, which is what a
      // desktop expects of anything that behaves like a button.
      // `accessibility.md`: "Let people use the keyboard alone to navigate and
      // interact with your app."
      row = Focus(
        focusNode: _focus,
        onFocusChange: (hasFocus) => setState(() => _focused = hasFocus),
        child: Actions(
          actions: <Type, Action<Intent>>{
            ActivateIntent: CallbackAction<ActivateIntent>(
              onInvoke: (_) {
                widget.onTap?.call();
                return null;
              },
            ),
          },
          child: Shortcuts(
            shortcuts: const <ShortcutActivator, Intent>{
              SingleActivator(LogicalKeyboardKey.enter): ActivateIntent(),
              SingleActivator(LogicalKeyboardKey.space): ActivateIntent(),
            },
            child: row,
          ),
        ),
      );
    }

    // A pressed state is already visible, but a *selected* row's only other cue
    // is a fill, which is why it is announced rather than left to colour.
    row = Semantics(
      button: interactive,
      selected: widget.selected,
      label: widget.semanticLabel,
      child: row,
    );

    final tooltip = widget.tooltip;
    if (tooltip != null) {
      row = Tooltip(message: tooltip, child: row);
    }
    return row;
  }
}
