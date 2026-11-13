import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'palette.dart';

/// Fades and blurs scrolling content where it meets a bar above it.
///
/// `layout.md`: "Instead of applying a solid or semi-opaque background color
/// beneath controls, use a scroll edge effect to visually elevate controls above
/// content." Without it, a page heading and a scrolled list collide at the
/// boundary, which is the thing the effect exists to prevent.
///
/// This is the *edge* only: it draws nothing under the bar itself, so the
/// material above stays responsible for its own appearance. It is placed over
/// the top of the content area and ignores pointer events, so it never
/// intercepts a scroll or a click.
class ScrollEdgeFade extends StatelessWidget {
  const ScrollEdgeFade({super.key, this.height = 20, this.blur = 8});

  /// How tall the fade and blur band is.
  final double height;

  /// Blur sigma at the top edge, tapering to nothing at the bottom of the band.
  final double blur;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    // An opaque-only fallback under increased contrast: a blur is a decorative
    // separation, and the setting exists to remove exactly that in favour of a
    // hard boundary.
    final highContrast = MediaQuery.maybeHighContrastOf(context) ?? false;

    return IgnorePointer(
      child: SizedBox(
        height: height,
        child: highContrast
            ? DecoratedBox(
                decoration: BoxDecoration(
                  color: palette.canvas,
                  border: Border(bottom: BorderSide(color: palette.separator, width: 0.5)),
                ),
              )
            : ClipRect(
                child: BackdropFilter(
                  filter: ui.ImageFilter.blur(sigmaX: blur, sigmaY: blur),
                  child: ShaderMask(
                    // A vertical gradient that is opaque at the top and clear at
                    // the bottom, so the blur fades into the content rather than
                    // ending on a line.
                    shaderCallback: (bounds) => LinearGradient(
                      begin: Alignment.topCenter,
                      end: Alignment.bottomCenter,
                      colors: [
                        palette.canvas.withValues(alpha: 0.85),
                        palette.canvas.withValues(alpha: 0),
                      ],
                    ).createShader(bounds),
                    blendMode: BlendMode.dstIn,
                    child: const ColoredBox(color: Colors.white),
                  ),
                ),
              ),
      ),
    );
  }
}
