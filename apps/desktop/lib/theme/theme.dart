import 'package:flutter/material.dart';

import 'palette.dart';
import 'tokens.dart';

export 'glass.dart';
export 'palette.dart';
export 'tokens.dart';

/// Builds the application theme.
///
/// The theme is intentionally thin: most visual work lives in the widgets, and
/// this sets the colours and text styles they inherit. Keeping colours in a
/// [SosumiPalette] extension rather than in `ColorScheme` means the glass tints,
/// which are not part of Material's scheme, have a home.
ThemeData buildTheme(Brightness brightness) {
  final palette = SosumiPalette.of(brightness);
  final isDark = brightness == Brightness.dark;

  final base = ThemeData(
    useMaterial3: true,
    brightness: brightness,
    scaffoldBackgroundColor: Colors.transparent,
    canvasColor: Colors.transparent,
    splashFactory: NoSplash.splashFactory,
    highlightColor: Colors.transparent,
    hoverColor: Colors.transparent,
    colorScheme: ColorScheme.fromSeed(
      seedColor: palette.accent,
      brightness: brightness,
    ).copyWith(surface: palette.canvas),
  );

  return base.copyWith(
    extensions: [palette],
    textTheme: base.textTheme.apply(
      bodyColor: palette.textPrimary,
      displayColor: palette.textPrimary,
    ),
    iconTheme: IconThemeData(color: palette.textSecondary, size: 18),
    dividerTheme: DividerThemeData(color: palette.separator, thickness: 0.5, space: 0.5),
    scrollbarTheme: ScrollbarThemeData(
      thickness: const WidgetStatePropertyAll(6),
      radius: const Radius.circular(Radii.pill),
      thumbColor: WidgetStatePropertyAll(palette.textSecondary.withValues(alpha: 0.3)),
      crossAxisMargin: 2,
    ),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 500),
      decoration: BoxDecoration(
        color: isDark ? const Color(0xF01C1C1E) : const Color(0xF0FFFFFF),
        borderRadius: BorderRadius.circular(Radii.small),
        border: Border.all(color: palette.glassBorder, width: 0.5),
      ),
      textStyle: TextStyles.caption.copyWith(color: palette.textPrimary),
    ),
    sliderTheme: SliderThemeData(
      trackHeight: 4,
      activeTrackColor: palette.textPrimary,
      inactiveTrackColor: palette.textSecondary.withValues(alpha: 0.25),
      thumbColor: palette.textPrimary,
      overlayShape: SliderComponentShape.noOverlay,
      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 0),
      trackShape: const RectangularSliderTrackShape(),
    ),
  );
}
