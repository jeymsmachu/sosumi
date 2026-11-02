import 'package:flutter/material.dart';

/// A top-level section in the app shell.
enum AppSection {
  home,
  browse,
  radio,
  library;

  String get label => switch (this) {
    AppSection.home => 'Home',
    AppSection.browse => 'Browse',
    AppSection.radio => 'Radio',
    AppSection.library => 'Library',
  };

  IconData get icon => switch (this) {
    AppSection.home => Icons.home_rounded,
    AppSection.browse => Icons.explore_rounded,
    AppSection.radio => Icons.radio_rounded,
    AppSection.library => Icons.library_music_rounded,
  };
}

/// An accent option exposed in Settings.
class AccentChoice {
  const AccentChoice(this.label, this.color);

  final String label;
  final Color color;
}

const accentChoices = <AccentChoice>[
  AccentChoice('Violet', Color(0xFF9B7BFF)),
  AccentChoice('Sky', Color(0xFF58C7FF)),
  AccentChoice('Amber', Color(0xFFFFB454)),
  AccentChoice('Mint', Color(0xFF69E3B0)),
];

/// App-wide UI state. Playback state will live in the Rust core later.
class SosumiController extends ChangeNotifier {
  AppSection _section = AppSection.home;
  bool _glassEnabled = true;
  bool _motionEnabled = true;
  bool _compactDensity = false;
  Color _accent = accentChoices.first.color;

  AppSection get section => _section;
  bool get glassEnabled => _glassEnabled;
  bool get motionEnabled => _motionEnabled;
  bool get compactDensity => _compactDensity;
  Color get accent => _accent;

  void selectSection(AppSection section) {
    if (_section == section) return;
    _section = section;
    notifyListeners();
  }

  void setGlassEnabled(bool value) {
    if (_glassEnabled == value) return;
    _glassEnabled = value;
    notifyListeners();
  }

  void setMotionEnabled(bool value) {
    if (_motionEnabled == value) return;
    _motionEnabled = value;
    notifyListeners();
  }

  void setCompactDensity(bool value) {
    if (_compactDensity == value) return;
    _compactDensity = value;
    notifyListeners();
  }

  void setAccent(Color color) {
    if (_accent == color) return;
    _accent = color;
    notifyListeners();
  }
}

/// Makes the controller available to the widget tree.
class SosumiScope extends InheritedNotifier<SosumiController> {
  const SosumiScope({
    super.key,
    required SosumiController controller,
    required super.child,
  }) : super(notifier: controller);

  static SosumiController of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<SosumiScope>();
    assert(scope != null, 'SosumiScope is missing from the widget tree');
    return scope!.notifier!;
  }
}
