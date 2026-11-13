import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:sosumi/theme/theme.dart';

void main() {
  group('theme', () {
    test('dark and light palettes differ', () {
      expect(SosumiPalette.dark.canvas, isNot(SosumiPalette.light.canvas));
    });

    test('of() selects by brightness', () {
      expect(SosumiPalette.of(Brightness.dark), SosumiPalette.dark);
      expect(SosumiPalette.of(Brightness.light), SosumiPalette.light);
    });

    testWidgets('theme carries the palette extension', (tester) async {
      final theme = buildTheme(Brightness.dark);
      expect(theme.extension<SosumiPalette>(), isNotNull);
    });
  });

  group('palette contrast', () {
    test('primary text contrasts with the canvas', () {
      // A rough luminance check: the foreground must be clearly lighter or
      // darker than the background, not merely different.
      const palette = SosumiPalette.dark;
      expect(
        (palette.textPrimary.computeLuminance() - palette.canvas.computeLuminance()).abs(),
        greaterThan(0.3),
      );
    });
  });
}
