import 'package:flutter/material.dart';

import 'palette.dart';
import 'tokens.dart';

/// A grouped container for content.
///
/// This is the counterpart to [GlassSurface], and the distinction matters. Apple
/// splits the interface into two layers:
///
/// - **Functional** — sidebars, toolbars, the transport. These float above
///   content and use Liquid Glass.
/// - **Content** — text, lists, media, and the grouped cards that hold settings
///   or track lists. These use standard materials, never glass.
///
/// Putting glass on a content card is a defect: it blurs the hierarchy the
/// material exists to create, and it costs a backdrop filter on every frame for
/// no gain. This widget is the plain, solid surface content should use.
class ContentSurface extends StatelessWidget {
  const ContentSurface({
    super.key,
    required this.child,
    this.radius = Radii.large,
    this.padding,
    this.margin,
    this.elevated = false,
  });

  final Widget child;
  final double radius;
  final EdgeInsetsGeometry? padding;
  final EdgeInsetsGeometry? margin;

  /// Raises the surface slightly, for a card that sits above the canvas.
  final bool elevated;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      margin: margin,
      padding: padding,
      decoration: BoxDecoration(
        // Apple's content surfaces are translucent white or black over the
        // canvas rather than a distinct grey, which keeps them reading as part
        // of the same background while still separating the group.
        color: elevated ? palette.fill : palette.fill.withValues(alpha: palette.fill.a * 0.5),
        borderRadius: BorderRadius.circular(radius),
        border: Border.all(color: palette.separator, width: 0.5),
      ),
      child: child,
    );
  }
}
