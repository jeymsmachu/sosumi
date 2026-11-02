import 'package:flutter/material.dart';

import 'state/sosumi_controller.dart';
import 'theme/sosumi_theme.dart';
import 'ui/shell.dart';

/// Root widget. Owns the app controller and rebuilds the theme when the accent
/// color changes.
class SosumiApp extends StatefulWidget {
  const SosumiApp({super.key});

  @override
  State<SosumiApp> createState() => _SosumiAppState();
}

class _SosumiAppState extends State<SosumiApp> {
  late final SosumiController _controller;

  @override
  void initState() {
    super.initState();
    _controller = SosumiController();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return SosumiScope(
      controller: _controller,
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, _) {
          return MaterialApp(
            title: 'sosumi',
            debugShowCheckedModeBanner: false,
            theme: buildSosumiTheme(accent: _controller.accent),
            home: const Shell(),
          );
        },
      ),
    );
  }
}
