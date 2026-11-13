import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../rust/api.dart';
import 'engine.dart';

/// One optional capability, in the shape the UI renders.
///
/// A projection of the bridge's `CapabilityDto` into something with the state as
/// a value rather than a string, so a typo in a comparison cannot silently make a
/// capability look unsupported.
class CapabilityInfo {
  const CapabilityInfo({
    required this.id,
    required this.name,
    required this.state,
    required this.detail,
    this.action,
    this.missing = const [],
    this.ready = false,
  });

  /// Stable machine id, for example `fairplay`.
  final String id;

  /// Display name, for example "Lossless & Hi-Res".
  final String name;

  /// One of `ready`, `available`, `degraded`, or `unsupported`.
  final String state;

  /// One line explaining the state.
  final String detail;

  /// What the user can do, when there is something to do.
  final String? action;

  /// Files the payload is missing, for an installable tier.
  final List<String> missing;

  /// True when the tier is installed and working.
  final bool ready;

  /// Whether the user can act to change this state.
  bool get isActionable => state == 'available' || state == 'degraded';

  /// Whether this machine cannot run the capability at all.
  bool get isUnsupported => state == 'unsupported';

  /// Builds from the bridge's flat DTO.
  factory CapabilityInfo.fromDto(CapabilityDto dto) => CapabilityInfo(
        id: dto.id,
        name: dto.name,
        state: dto.state,
        detail: dto.detail,
        action: dto.action,
        missing: dto.missing,
        ready: dto.ready,
      );
}

/// Reads the capability list and performs installs.
///
/// An `AsyncNotifier` rather than a plain `StateNotifier` because the first read
/// crosses the bridge and can fail: the loading and error states are real, and a
/// settings screen that showed an empty list on a slow read would look like the
/// machine has no capabilities at all.
class CapabilitiesController extends AsyncNotifier<List<CapabilityInfo>> {
  @override
  Future<List<CapabilityInfo>> build() => _read();

  Future<List<CapabilityInfo>> _read() async {
    final engine = ref.watch(engineProvider).requireValue;
    final dtos = await engine.capabilities();
    return dtos.map(CapabilityInfo.fromDto).toList();
  }

  /// Installs a capability and refreshes the list.
  ///
  /// Only the half that needs no network runs here: the stubs are carried in the
  /// binary. Apple's own files cannot be embedded, so the refresh reports what is
  /// still missing instead of pretending the install finished.
  Future<void> install(String id) async {
    final engine = ref.watch(engineProvider).requireValue;

    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      if (id == 'fairplay') {
        await engine.installLosslessStubs();
      }
      return _read();
    });
  }

  /// Re-reads the list, for example after the user installs something elsewhere.
  Future<void> refresh() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(_read);
  }
}

/// The capability list.
final capabilitiesProvider =
    AsyncNotifierProvider<CapabilitiesController, List<CapabilityInfo>>(
  CapabilitiesController.new,
);
