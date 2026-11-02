import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:sosumi/main.dart';

void main() {
  setUpAll(() async {
    TestWidgetsFlutterBinding.ensureInitialized();
    await LiquidGlassWidgets.initialize(enablePerformanceMonitor: false);
  });

  testWidgets('renders the adaptive shell and opens settings', (tester) async {
    await tester.pumpWidget(const SosumiRoot(adaptiveQuality: false));
    await tester.pump(const Duration(milliseconds: 600));

    expect(find.text('sosumi'), findsWidgets);
    expect(find.text('Recently played'), findsOneWidget);
    expect(find.text('A quieter way to listen.'), findsOneWidget);

    await tester.tap(find.byTooltip('Settings').first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 500));

    expect(find.text('Settings'), findsOneWidget);
    expect(find.text('Glass effects'), findsOneWidget);
  });

  testWidgets('switches to the compact dock on a narrow viewport', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(500, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const SosumiRoot(adaptiveQuality: false));
    await tester.pump(const Duration(milliseconds: 600));

    expect(find.byKey(const Key('compact-dock')), findsOneWidget);
    expect(find.text('Home'), findsWidgets);
  });
}
