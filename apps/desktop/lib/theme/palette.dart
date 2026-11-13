import 'package:flutter/material.dart';

/// Sosumi's colour palette.
///
/// The values are Apple's own, read from the tokens the Apple Music web player
/// ships (`--systemPrimary`, `--systemSecondary`, `--systemRed`, and the
/// `systemGroupedBackground` family). Using the real numbers rather than
/// approximations is what makes the app sit beside Apple's software without
/// looking off.
///
/// Apple expresses most text colours as white with an alpha rather than a grey,
/// which is what lets the glass behind them show through. That convention is
/// kept here.
@immutable
class SosumiPalette extends ThemeExtension<SosumiPalette> {
  const SosumiPalette({
    required this.accent,
    required this.accentSoft,
    required this.blue,
    required this.glassFill,
    required this.glassBorder,
    required this.glassHighlight,
    required this.canvas,
    required this.sidebarFill,
    required this.playerFill,
    required this.elevatedGlass,
    required this.textPrimary,
    required this.textSecondary,
    required this.textTertiary,
    required this.separator,
    required this.fill,
    required this.selection,
  });

  /// The brand red. Apple Music's accent, `#fa233b` light / `#ff453a` dark.
  final Color accent;

  /// The accent at low alpha, for selection washes behind sidebar rows.
  final Color accentSoft;

  /// The system blue, for focus rings and links.
  final Color blue;

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

  /// A denser fill for glass presented over an already-dimmed backdrop, such as
  /// a dialog. Over a dimming layer the regular fill reads as almost nothing, so
  /// a dialog needs more body to hold text.
  final Color elevatedGlass;

  /// Primary label. Apple's `--systemPrimary`.
  final Color textPrimary;

  /// Secondary label. Apple's `--systemSecondary`.
  final Color textSecondary;

  /// Tertiary label, for the least important text. Apple's `--systemTertiary`.
  final Color textTertiary;

  final Color separator;

  /// A subtle fill for chips and inactive tracks. Apple's `--systemQuaternary`.
  final Color fill;

  /// The fill behind a selected sidebar row. Apple uses a low-alpha tint rather
  /// than the accent at full strength, so a selected row reads as chosen without
  /// shouting, and so it stays legible over any wallpaper behind the glass.
  final Color selection;

  /// The dark palette, which is the default.
  static const dark = SosumiPalette(
    accent: Color(0xFFFF453A),
    accentSoft: Color(0x24FF453A),
    blue: Color(0xFF0A84FF),
    glassFill: Color(0x14FFFFFF),
    glassBorder: Color(0x1FFFFFFF),
    glassHighlight: Color(0x40FFFFFF),
    canvas: Color(0xFF1C1C1E),
    sidebarFill: Color(0x0AFFFFFF),
    playerFill: Color(0x1AFFFFFF),
    elevatedGlass: Color(0xE61C1C1E),
    // Apple writes these as `#ffffffd9` (RRGGBBAA); Flutter wants AARRGGBB.
    textPrimary: Color(0xD9FFFFFF),
    textSecondary: Color(0x8CFFFFFF),
    textTertiary: Color(0x40FFFFFF),
    separator: Color(0x1FFFFFFF),
    fill: Color(0x14FFFFFF),
    selection: Color(0x1FFFFFFF),
  );

  /// The light palette.
  static const light = SosumiPalette(
    accent: Color(0xFFFA233B),
    accentSoft: Color(0x14FA233B),
    blue: Color(0xFF007AFF),
    glassFill: Color(0x80FFFFFF),
    glassBorder: Color(0x1A000000),
    glassHighlight: Color(0xD9FFFFFF),
    canvas: Color(0xFFF2F2F7),
    sidebarFill: Color(0x99FFFFFF),
    playerFill: Color(0xCCFFFFFF),
    elevatedGlass: Color(0xF2FFFFFF),
    // Apple's `#000000d9` (RRGGBBAA) as Flutter's AARRGGBB.
    textPrimary: Color(0xD9000000),
    textSecondary: Color(0x8C000000),
    textTertiary: Color(0x40000000),
    separator: Color(0x1F000000),
    fill: Color(0x0F000000),
    selection: Color(0x14000000),
  );

  /// The palette for a brightness.
  static SosumiPalette of(Brightness brightness) =>
      brightness == Brightness.dark ? dark : light;

  @override
  SosumiPalette copyWith({
    Color? accent,
    Color? accentSoft,
    Color? blue,
    Color? glassFill,
    Color? glassBorder,
    Color? glassHighlight,
    Color? canvas,
    Color? sidebarFill,
    Color? playerFill,
    Color? elevatedGlass,
    Color? textPrimary,
    Color? textSecondary,
    Color? textTertiary,
    Color? separator,
    Color? fill,
    Color? selection,
  }) {
    return SosumiPalette(
      accent: accent ?? this.accent,
      accentSoft: accentSoft ?? this.accentSoft,
      blue: blue ?? this.blue,
      glassFill: glassFill ?? this.glassFill,
      glassBorder: glassBorder ?? this.glassBorder,
      glassHighlight: glassHighlight ?? this.glassHighlight,
      canvas: canvas ?? this.canvas,
      sidebarFill: sidebarFill ?? this.sidebarFill,
      playerFill: playerFill ?? this.playerFill,
      elevatedGlass: elevatedGlass ?? this.elevatedGlass,
      textPrimary: textPrimary ?? this.textPrimary,
      textSecondary: textSecondary ?? this.textSecondary,
      textTertiary: textTertiary ?? this.textTertiary,
      separator: separator ?? this.separator,
      fill: fill ?? this.fill,
      selection: selection ?? this.selection,
    );
  }

  @override
  SosumiPalette lerp(ThemeExtension<SosumiPalette>? other, double t) {
    if (other is! SosumiPalette) return this;
    return SosumiPalette(
      accent: Color.lerp(accent, other.accent, t)!,
      accentSoft: Color.lerp(accentSoft, other.accentSoft, t)!,
      blue: Color.lerp(blue, other.blue, t)!,
      glassFill: Color.lerp(glassFill, other.glassFill, t)!,
      glassBorder: Color.lerp(glassBorder, other.glassBorder, t)!,
      glassHighlight: Color.lerp(glassHighlight, other.glassHighlight, t)!,
      canvas: Color.lerp(canvas, other.canvas, t)!,
      sidebarFill: Color.lerp(sidebarFill, other.sidebarFill, t)!,
      playerFill: Color.lerp(playerFill, other.playerFill, t)!,
      elevatedGlass: Color.lerp(elevatedGlass, other.elevatedGlass, t)!,
      textPrimary: Color.lerp(textPrimary, other.textPrimary, t)!,
      textSecondary: Color.lerp(textSecondary, other.textSecondary, t)!,
      textTertiary: Color.lerp(textTertiary, other.textTertiary, t)!,
      separator: Color.lerp(separator, other.separator, t)!,
      fill: Color.lerp(fill, other.fill, t)!,
      selection: Color.lerp(selection, other.selection, t)!,
    );
  }
}

/// Convenience access to the palette from any context.
extension PaletteContext on BuildContext {
  /// The active palette.
  SosumiPalette get palette =>
      Theme.of(this).extension<SosumiPalette>() ?? SosumiPalette.dark;
}
