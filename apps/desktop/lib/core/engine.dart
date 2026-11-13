import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../rust/api.dart';
import '../rust/frb_generated.dart';

/// The Rust engine and everything derived from it.
///
/// The engine is created once and lives for the process's lifetime, mirroring
/// the Rust side: one OS process, one engine, one Tokio runtime. Nothing here
/// spawns a second copy, and there is no daemon to reconnect to.
abstract final class RustEngine {
  static Engine? _engine;

  /// Initialises flutter_rust_bridge and creates the engine.
  ///
  /// Safe to call more than once; the engine is created only on the first call.
  static Future<Engine> ensure() async {
    final existing = _engine;
    if (existing != null) return existing;

    await RustLib.init();
    final engine = await createEngine();
    _engine = engine;
    return engine;
  }

  /// The engine, if it has been created.
  static Engine? get current => _engine;
}

/// Provides the engine to the widget tree.
///
/// Awaited once at the root; widgets that need it read this provider and never
/// touch the bridge directly.
final engineProvider = FutureProvider<Engine>((ref) async {
  return RustEngine.ensure();
});
