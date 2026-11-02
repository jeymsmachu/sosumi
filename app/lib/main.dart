import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'src/app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await LiquidGlassWidgets.initialize();
  runApp(const SosumiRoot());
}

/// Wraps the app in the Liquid Glass infrastructure.
///
/// The theme is fitted to iOS 27 glass; adaptive quality keeps older Android
/// devices and desktop GPUs on the lightweight shader path.
class SosumiRoot extends StatelessWidget {
  const SosumiRoot({super.key, this.adaptiveQuality = true});

  final bool adaptiveQuality;

  @override
  Widget build(BuildContext context) {
    return LiquidGlassWidgets.wrap(
      child: const SosumiApp(),
      brightnessResolver: Theme.maybeBrightnessOf,
      adaptiveQuality: adaptiveQuality,
      theme: const GlassThemeData.ios27(),
    );
  }
}
