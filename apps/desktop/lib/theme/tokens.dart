import 'package:flutter/material.dart';

/// Layout, shape, and motion constants.
///
/// The radii are Apple's: their CSS uses 10, 16, 20 and fully-rounded pills, and
/// the nested-radius rule (a container inside another uses a smaller radius) is
/// what keeps curves concentric.
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

/// Corner radii.
abstract final class Radii {
  /// Small controls: chips, badges.
  static const double small = 6;

  /// The sidebar row and similar controls. Apple uses 10 here.
  static const double row = 10;

  /// Cards and list rows.
  static const double large = 16;

  /// Panels and sheets. Apple uses 20 for these.
  static const double panel = 20;

  /// The floating player bar. Apple uses 24 for its floating transport.
  static const double player = 24;

  /// A pill: fully rounded ends.
  static const double pill = 999;
}

/// Durations and curves.
///
/// Apple's own guidance is that motion should be quick and unhurried: 0.2-0.3 s
/// for a control, longer for a panel. The curves are the standard ease-out for
/// arrival and ease-in for departure, with a slight overshoot reserved for
/// controls settling.
abstract final class Motion {
  /// Hover and press feedback.
  static const Duration fast = Duration(milliseconds: 120);

  /// Panel and colour transitions.
  static const Duration normal = Duration(milliseconds: 220);

  /// Full-screen transitions and reveals.
  static const Duration slow = Duration(milliseconds: 380);

  /// Elements arriving on screen.
  static const Curve enter = Curves.easeOutCubic;

  /// Elements leaving.
  static const Curve exit = Curves.easeInCubic;

  /// A control returning to rest, with the small overshoot Apple uses.
  static const Curve settle = Curves.easeOutBack;
}

/// Fixed sizes several screens must agree on.
abstract final class Sizes {
  /// The sidebar width.
  static const double sidebarWidth = 220;

  /// The sidebar width when only icons are shown.
  static const double sidebarCollapsedWidth = 60;

  /// A sidebar row's height.
  static const double sidebarRowHeight = 30;

  /// The floating player's height.
  static const double playerHeight = 64;

  /// A standard list artwork in a track row. Apple uses 40.
  static const double listArtwork = 40;

  /// Grid artwork.
  static const double gridArtwork = 160;

  /// The large artwork on a detail page.
  static const double heroArtwork = 240;

  /// The soft edge inset for the floating player, so it reads as a floating
  /// capsule rather than a docked bar.
  static const double playerInset = 12;
}

/// The type scale.
///
/// Sizes and weights are Apple's macOS values, which the web player's CSS also
/// uses: a 13 px body, a 15/17/22 title ramp, and 10 px captions. Allowing the
/// platform to render at these sizes rather than scaling them up is what makes
/// the app look native instead of like a web page in a window.
///
/// Line heights are Apple's, expressed as a multiple for Flutter.
abstract final class TextStyles {
  /// A hero title, such as a page header. 26/32 semibold.
  static const TextStyle largeTitle = TextStyle(
    fontSize: 26,
    fontWeight: FontWeight.w700,
    letterSpacing: 0.36,
    height: 1.23,
  );

  /// Title 1. 22/26.
  static const TextStyle title1 = TextStyle(
    fontSize: 22,
    fontWeight: FontWeight.w700,
    letterSpacing: -0.26,
    height: 1.18,
  );

  /// Title 2, for section headings. 17/22.
  static const TextStyle title2 = TextStyle(
    fontSize: 17,
    fontWeight: FontWeight.w600,
    letterSpacing: -0.43,
    height: 1.29,
  );

  /// Title 3. 15/20.
  static const TextStyle title3 = TextStyle(
    fontSize: 15,
    fontWeight: FontWeight.w600,
    letterSpacing: -0.24,
    height: 1.33,
  );

  /// Headline: 13/16 semibold. Used for emphasised body text.
  static const TextStyle headline = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w600,
    letterSpacing: -0.08,
    height: 1.23,
  );

  /// Body: 13/16 regular. The default for essentially all text.
  static const TextStyle body = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w400,
    letterSpacing: -0.08,
    height: 1.23,
  );

  /// Callout: 12/15.
  static const TextStyle callout = TextStyle(
    fontSize: 12,
    fontWeight: FontWeight.w400,
    letterSpacing: 0,
    height: 1.25,
  );

  /// Subheadline: 11/14.
  static const TextStyle subheadline = TextStyle(
    fontSize: 11,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.07,
    height: 1.27,
  );

  /// Footnote: 10/13.
  static const TextStyle footnote = TextStyle(
    fontSize: 10,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.12,
    height: 1.3,
  );

  /// Caption 1: 10/13.
  static const TextStyle caption1 = TextStyle(
    fontSize: 10,
    fontWeight: FontWeight.w400,
    letterSpacing: 0.12,
    height: 1.3,
  );

  /// Caption 2: 10/13 medium. For the smallest labels, such as a badge.
  static const TextStyle caption2 = TextStyle(
    fontSize: 10,
    fontWeight: FontWeight.w500,
    letterSpacing: 0.12,
    height: 1.3,
  );

  /// A numeric readout. Tabular figures keep a changing timecode from shifting.
  static const TextStyle numeric = TextStyle(
    fontSize: 11,
    fontWeight: FontWeight.w500,
    letterSpacing: 0.07,
    height: 1.27,
    fontFeatures: [FontFeature.tabularFigures()],
  );

  /// An emphasised row title, for the playing track in a list.
  static const TextStyle rowTitle = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w600,
    letterSpacing: -0.08,
    height: 1.23,
  );
}
