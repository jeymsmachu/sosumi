import 'package:flutter/material.dart';

import 'palette.dart';
import 'tokens.dart';

/// A thin, rounded progress track.
///
/// Material's `Slider` paints a thumb, an overlay, and a discrete-step track,
/// none of which belong in a media transport: Apple's scrubber is a hairline
/// capsule with no visible thumb at rest. Building it directly is also cheaper
/// than fighting four layers of `SliderTheme`.
class Scrubber extends StatelessWidget {
  const Scrubber({
    super.key,
    required this.value,
    this.onChanged,
    this.height = 3,
    this.activeColor,
    this.trackColor,
  });

  /// Progress in the range `0..1`.
  final double value;

  /// Called while dragging. A `null` callback renders the track as read-only.
  final ValueChanged<double>? onChanged;

  /// Track thickness.
  final double height;

  final Color? activeColor;
  final Color? trackColor;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final clamped = value.clamp(0.0, 1.0);

    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final filled = width * clamped;

        void seek(Offset local) {
          if (onChanged == null || width <= 0) return;
          onChanged!((local.dx / width).clamp(0.0, 1.0));
        }

        return MouseRegion(
          cursor: onChanged == null ? MouseCursor.defer : SystemMouseCursors.click,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapDown: (d) => seek(d.localPosition),
            onHorizontalDragUpdate: (d) => seek(d.localPosition),
            child: SizedBox(
              height: 16,
              child: Center(
                child: Stack(
                  children: [
                    Container(
                      width: width,
                      height: height,
                      decoration: BoxDecoration(
                        color: trackColor ?? palette.textTertiary,
                        borderRadius: BorderRadius.circular(Radii.pill),
                      ),
                    ),
                    Container(
                      width: filled,
                      height: height,
                      decoration: BoxDecoration(
                        color: activeColor ?? palette.textSecondary,
                        borderRadius: BorderRadius.circular(Radii.pill),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
