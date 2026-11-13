import 'package:flutter/material.dart';

import 'palette.dart';
import 'tokens.dart';

export 'content_surface.dart';
export 'glass.dart';
export 'motion.dart';
export 'palette.dart';
export 'reveal.dart';
export 'scroll_edge.dart';
export 'scrubber.dart';
export 'tokens.dart';

/// The font family stack.
///
/// Apple's own stack, in the order their CSS uses it. `-apple-system` and
/// `BlinkMacSystemFont` resolve to SF Pro on Apple platforms; elsewhere the
/// stack falls through to Inter, which is the closest metrically-compatible
/// relative available on Linux, then to whatever the desktop provides.
///
/// Bundling SF Pro is not an option: Apple licenses it for use on Apple
/// platforms only, and redistributing it would breach that. Inter is the honest
/// substitute, and it is credited in the README.
const List<String> kFontStack = [
  // Apple platforms.
  '-apple-system',
  'BlinkMacSystemFont',
  'SF Pro Text',
  'SF Pro Display',
  'SF Pro',
  // Linux substitute, then the platform default.
  'Inter',
  'Inter Variable',
  'Roboto',
  'Noto Sans',
  'Helvetica Neue',
  'Helvetica',
  'Arial',
  'sans-serif',
];

/// Applies the font stack to a style.
TextStyle _withFont(TextStyle style) =>
    style.copyWith(fontFamily: kFontStack.first, fontFamilyFallback: kFontStack.skip(1).toList());

/// Builds the application theme.
///
/// Two things matter more than the colours here:
///
/// 1. **A root `DefaultTextStyle`.** Without one, Flutter falls back to
///    `DefaultTextStyle.fallback()`, which paints text in yellow with a double
///    underline — the framework's way of saying "you forgot an ancestor". Every
///    `Text` in the app inherits this instead.
/// 2. **A `textTheme` built from the macOS scale.** Apple's sizes are small
///    (13 pt body), so building them explicitly rather than letting Material
///    default to its own, larger ramp is what makes the app look native.
ThemeData buildTheme(Brightness brightness) {
  final palette = SosumiPalette.of(brightness);
  final isDark = brightness == Brightness.dark;

  final base = ThemeData(
    useMaterial3: true,
    brightness: brightness,
    // The window is drawn by the app, not by Material, so the surfaces that
    // would normally paint a background are transparent.
    scaffoldBackgroundColor: Colors.transparent,
    canvasColor: Colors.transparent,
    splashFactory: NoSplash.splashFactory,
    highlightColor: Colors.transparent,
    hoverColor: Colors.transparent,
    colorScheme: ColorScheme.fromSeed(seedColor: palette.accent, brightness: brightness)
        .copyWith(
          surface: palette.canvas,
          primary: palette.accent,
          outline: palette.separator,
        ),
  );

  // Apple's macOS text styles, expressed as Material's ramp. Sizes and weights
  // are taken from Apple's own specification table.
  final text = TextTheme(
    displayLarge: _withFont(TextStyles.largeTitle),
    displayMedium: _withFont(TextStyles.title1),
    displaySmall: _withFont(TextStyles.title1),
    headlineLarge: _withFont(TextStyles.title1),
    headlineMedium: _withFont(TextStyles.title2),
    headlineSmall: _withFont(TextStyles.title3),
    titleLarge: _withFont(TextStyles.title2),
    titleMedium: _withFont(TextStyles.headline),
    titleSmall: _withFont(TextStyles.headline),
    bodyLarge: _withFont(TextStyles.body),
    bodyMedium: _withFont(TextStyles.body),
    bodySmall: _withFont(TextStyles.callout),
    labelLarge: _withFont(TextStyles.headline),
    labelMedium: _withFont(TextStyles.subheadline),
    labelSmall: _withFont(TextStyles.caption1),
  ).apply(bodyColor: palette.textPrimary, displayColor: palette.textPrimary);

  return base.copyWith(
    extensions: [palette],
    textTheme: text,
    primaryTextTheme: text,
    iconTheme: IconThemeData(color: palette.textSecondary, size: 16),
    dividerTheme: DividerThemeData(color: palette.separator, thickness: 0.5, space: 0.5),
    scrollbarTheme: ScrollbarThemeData(
      thickness: const WidgetStatePropertyAll(6),
      radius: const Radius.circular(Radii.pill),
      thumbColor: WidgetStatePropertyAll(palette.textTertiary),
      crossAxisMargin: 2,
    ),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 400),
      decoration: BoxDecoration(
        color: isDark ? const Color(0xF01C1C1E) : const Color(0xF0FFFFFF),
        borderRadius: BorderRadius.circular(Radii.small),
        border: Border.all(color: palette.glassBorder, width: 0.5),
      ),
      textStyle: _withFont(TextStyles.caption1).copyWith(color: palette.textPrimary),
    ),
    sliderTheme: SliderThemeData(
      trackHeight: 4,
      activeTrackColor: palette.textSecondary,
      inactiveTrackColor: palette.textTertiary,
      thumbColor: palette.textPrimary,
      overlayShape: SliderComponentShape.noOverlay,
      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
      trackShape: const RoundedRectSliderTrackShape(),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(
      color: palette.accent,
      linearTrackColor: palette.textTertiary,
    ),
  );
}
