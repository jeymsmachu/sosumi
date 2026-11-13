import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app/app.dart';

/// The entry point.
///
/// The window is translucent so the desktop compositor can blur what is behind
/// it, which is what makes the glass read as glass rather than as a grey panel.
/// Where the compositor cannot do that, [`buildTheme`]'s canvas colour is the
/// fallback and the UI still reads correctly.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  runApp(const ProviderScope(child: SosumiApp()));
}
