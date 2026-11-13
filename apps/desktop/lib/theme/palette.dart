import 'package:flutter/material.dart';

/// The application's colour palette.
///
/// These are Sosumi's own values, tuned to sit beside Apple's design language
/// without copying Apple's assets. There is no Apple logo, wordmark, or trade
/// dress anywhere in the app; see `docs/LEGAL-SCOPE.md`.
@immutable
class SosumiPalette extends ThemeExtension<SosumiPalette> {
  const SosumiPalette({
    required this.accent,
    required this.accentSoft,
    required this.glassFill,
    required this.glassBorder,
    required this.glassHighlight,
    required this.canvas,
    required this.sidebarFill,
    required this.playerFill,
    required this.textPrimary,
    required this.textSecondary,
    required this.separator,
  });

  /// The primary accent, used for the playing indicator and active controls.
  final Color accent;

  /// A translucent accent for selection washes.
  final Color accentSoft;

  /// The body tint of a glass surface.
  final Color glassFill;

  /// The hairline border of a glass surface.
  final Color glassBorder;

  /// The specular highlight along a glass edge.
  final Color glassHighlight;

  /// The window background behind the glass.
  final Color canvas;

  /// The sidebar's base tint.
  final Color sidebarFill;

  /// The floating player's base tint.
  final Color playerFill;

  final Color textPrimary;
  final Color textSecondary;
  final Color separator;

  /// The dark palette, which is the default: the glass reads best over a deep
  /// canvas, and it matches the platform's dark appearance.
  static const dark = SosumiPalette(
    accent: Color(0xFFFA586A),
    accentSoft: Color(0x33FA586A),
    glassFill: Color(0x14FFFFFF),
    glassBorder: Color(0x1FFFFFFF),
    glassHighlight: Color(0x40FFFFFF),
    canvas: Color(0xFF0B0B0D),
    sidebarFill: Color(0x0DFFFFFF),
    playerFill: Color(0x1AFFFFFF),
    textPrimary: Color(0xFFF5F5F7),
    textSecondary: Color(0x99EBEBF5),
    separator: Color(0x1AFFFFFF),
  );

  /// The light palette, for users who prefer it.
  static const light = SosumiPalette(
    accent: Color(0xFFFA2D48),
    accentSoft: Color(0x1FFA2D48),
    glassFill: Color(0x66FFFFFF),
    glassBorder: Color(0x1F000000),
    glassHighlight: Color(0xCCFFFFFF),
    canvas: Color(0xFFF2F2F7),
    sidebarFill: Color(0x99FFFFFF),
    playerFill: Color(0xB3FFFFFF),
    textPrimary: Color(0xFF1D1D1F),
    textSecondary: Color(0x993C3C43),
    separator: Color(0x1F000000),
  );

  /// The palette for a brightness.
  static SosumiPalette of(Brightness brightness) =>
      brightness == Brightness.dark ? dark : light;

  @override
  SosumiPalette copyWith({
    Color? accent,
    Color? accentSoft,
    Color? glassFill,
    Color? glassBorder,
    Color? glassHighlight,
    Color? canvas,
    Color? sidebarFill,
    Color? playerFill,
    Color? textPrimary,
    Color? textSecondary,
    Color? separator,
  }) {
    return SosumiPalette(
      accent: accent ?? this.accent,
      accentSoft: accentSoft ?? this.accentSoft,
      glassFill: glassFill ?? this.glassFill,
      glassBorder: glassBorder ?? this.glassBorder,
      glassHighlight: glassHighlight ?? this.glassHighlight,
      canvas: canvas ?? this.canvas,
      sidebarFill: sidebarFill ?? this.sidebarFill,
      playerFill: playerFill ?? this.playerFill,
      textPrimary: textPrimary ?? this.textPrimary,
      textSecondary: textSecondary ?? this.textSecondary,
      separator: separator ?? this.separator,
    );
  }

  @override
  SosumiPalette lerp(ThemeExtension<SosumiPalette>? other, double t) {
    if (other is! SosumiPalette) return this;
    return SosumiPalette(
      accent: Color.lerp(accent, other.accent, t)!,
      accentSoft: Color.lerp(accentSoft, other.accentSoft, t)!,
      glassFill: Color.lerp(glassFill, other.glassFill, t)!,
      glassBorder: Color.lerp(glassBorder, other.glassBorder, t)!,
      glassHighlight: Color.lerp(glassHighlight, other.glassHighlight, t)!,
      canvas: Color.lerp(canvas, other.canvas, t)!,
      sidebarFill: Color.lerp(sidebarFill, other.sidebarFill, t)!,
      playerFill: Color.lerp(playerFill, other.playerFill, t)!,
      textPrimary: Color.lerp(textPrimary, other.textPrimary, t)!,
      textSecondary: Color.lerp(textSecondary, other.textSecondary, t)!,
      separator: Color.lerp(separator, other.separator, t)!,
    );
  }
}

/// Convenience access to the palette from any context.
extension PaletteContext on BuildContext {
  /// The active palette.
  SosumiPalette get palette =>
      Theme.of(this).extension<SosumiPalette>() ?? SosumiPalette.dark;
}
