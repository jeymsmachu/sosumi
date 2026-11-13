import 'package:flutter/material.dart';

/// Layout and shape constants.
///
/// One place for the values that make the app feel consistent: corner radii,
/// spacing steps, and the control sizes that line up across screens.
abstract final class Insets {
  /// The smallest gap, between an icon and its label.
  static const double xs = 4;

  /// A tight gap, between related lines of text.
  static const double sm = 8;

  /// The standard gap between elements in a group.
  static const double md = 12;

  /// The gap between groups.
  static const double lg = 20;

  /// The gap between major regions.
  static const double xl = 32;

  /// The horizontal page margin.
  static const double page = 24;
}

/// Corner radii, following the nested-radius idea: a container inside another
/// uses a smaller radius so the curves stay concentric.
abstract final class Radii {
  /// Small controls: chips, badges.
  static const double small = 8;

  /// Standard controls: buttons, fields.
  static const double medium = 12;

  /// Cards and list rows.
  static const double large = 16;

  /// Panels and sheets.
  static const double panel = 20;

  /// The floating player bar.
  static const double player = 24;

  /// A pill: fully rounded ends.
  static const double pill = 999;
}

/// Durations and curves for motion.
abstract final class Motion {
  /// Hover and tap feedback. Short enough to feel immediate.
  static const Duration fast = Duration(milliseconds: 120);

  /// Panel and colour transitions.
  static const Duration normal = Duration(milliseconds: 240);

  /// Full-screen transitions and reveals.
  static const Duration slow = Duration(milliseconds: 420);

  /// The curve for elements arriving on screen.
  static const Curve enter = Curves.easeOutCubic;

  /// The curve for elements leaving.
  static const Curve exit = Curves.easeInCubic;

  /// The curve for a control returning to rest.
  static const Curve settle = Curves.easeOutBack;
}

/// Fixed sizes that several screens must agree on.
abstract final class Sizes {
  /// The sidebar's default width.
  static const double sidebarWidth = 232;

  /// The sidebar's compact width, when collapsed.
  static const double sidebarCollapsedWidth = 68;

  /// The floating player's height.
  static const double playerHeight = 72;

  /// A standard list artwork.
  static const double listArtwork = 40;

  /// Grid artwork.
  static const double gridArtwork = 160;

  /// The large artwork on a detail page.
  static const double heroArtwork = 240;

  /// The title-bar height, for the drag region.
  static const double titleBarHeight = 44;
}

/// TextStyles.
///
/// Sizes and weights follow Apple's scale closely enough to feel native. The
/// font is the platform's: Flutter resolves it to the system UI font, which on
/// Linux is whatever the user configured.
abstract final class TextStyles {
  /// A page title.
  static const TextStyle largeTitle = TextStyle(
    fontSize: 28,
    fontWeight: FontWeight.w700,
    letterSpacing: -0.4,
    height: 1.15,
  );

  /// A section heading.
  static const TextStyle title = TextStyle(
    fontSize: 20,
    fontWeight: FontWeight.w700,
    letterSpacing: -0.2,
    height: 1.2,
  );

  /// A group heading inside a page.
  static const TextStyle headline = TextStyle(
    fontSize: 15,
    fontWeight: FontWeight.w600,
    height: 1.3,
  );

  /// Body copy.
  static const TextStyle body = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w400,
    height: 1.35,
  );

  /// A list row's primary line.
  static const TextStyle rowTitle = TextStyle(
    fontSize: 13.5,
    fontWeight: FontWeight.w500,
    height: 1.3,
  );

  /// A list row's secondary line.
  static const TextStyle rowSubtitle = TextStyle(
    fontSize: 12,
    fontWeight: FontWeight.w400,
    height: 1.3,
  );

  /// Labels on controls.
  static const TextStyle control = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w500,
    height: 1.2,
  );

  /// A small caption or badge.
  static const TextStyle caption = TextStyle(
    fontSize: 11,
    fontWeight: FontWeight.w500,
    letterSpacing: 0.2,
    height: 1.2,
  );

  /// A numeric readout, such as a timecode.
  static const TextStyle numeric = TextStyle(
    fontSize: 11,
    fontWeight: FontWeight.w500,
    fontFeatures: [FontFeature.tabularFigures()],
    height: 1.2,
  );
}
