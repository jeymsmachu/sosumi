import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/capabilities.dart';
import '../../core/engine.dart';
import '../../core/install.dart';
import '../../core/session.dart';
import '../../theme/theme.dart';
import '../account/account_row.dart' show showSignIn;
import '../account/install_progress.dart';

/// The Settings page.
///
/// Leads with the account, then the sound-quality choice, then the optional
/// capabilities. That order matches what a person comes here for: who they are,
/// what they hear, and what else they could have.
///
/// `settings.md`: "Minimize the number of settings you offer"; "Put general,
/// infrequently changed settings in your custom settings area". Every row here
/// is one of those, and capability state is read live rather than shipped as a
/// list, so the screen cannot claim something is installed when it is not. See
/// [capabilitiesProvider].
class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final capabilities = ref.watch(capabilitiesProvider);

    return ListView(
      // Bottom padding clears the floating player, which overlays the content.
      padding: const EdgeInsets.fromLTRB(
        Insets.page,
        0,
        Insets.page,
        Sizes.playerHeight + Sizes.playerInset * 2,
      ),
      children: [
        _Group(
          title: 'Account',
          children: [_AccountRow(signedIn: session.signedIn)],
        ),
        const SizedBox(height: Insets.lg),
        const _Group(
          title: 'Audio Quality',
          children: [_QualityPicker()],
        ),
        const SizedBox(height: Insets.lg),
        _Group(
          title: 'Enhancements',
          children: [
            switch (capabilities) {
              AsyncData(:final value) => _CapabilityList(capabilities: value),
              AsyncError(:final error) => _CapabilityError(message: '$error'),
              _ => const _CapabilityLoading(),
            },
          ],
        ),
        const SizedBox(height: Insets.xl),
        const _About(),
      ],
    );
  }
}

/// A titled group of rows, drawn as one content surface.
///
/// Content surfaces, not glass: Apple's rule is that the material belongs to the
/// functional layer, so a settings card uses a standard surface. See
/// `materials.md`: "Don't use Liquid Glass in the content layer."
class _Group extends StatelessWidget {
  const _Group({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: Insets.md, bottom: Insets.sm),
          child: Text(
            // Sentence case rather than an all-caps eyebrow: `writing.md` asks
            // for consistent capitalization, and the rest of the app uses it.
            title,
            style: TextStyles.subheadline.copyWith(
              color: palette.textSecondary,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        ContentSurface(
          radius: Radii.large,
          elevated: true,
          padding: const EdgeInsets.symmetric(horizontal: Insets.lg, vertical: Insets.md),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: children),
        ),
      ],
    );
  }
}

class _AccountRow extends ConsumerWidget {
  const _AccountRow({required this.signedIn});

  final bool signedIn;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final palette = context.palette;

    if (!signedIn) {
      // An invitation rather than a status line, per `writing.md`: "Empty screens
      // invite the next action." The action is the same function the account menu
      // calls, so there is one sign-in flow rather than two that can drift.
      return _SettingRow(
        title: 'Not signed in',
        subtitle: 'An active Apple Music subscription is required',
        trailing: FilledButton(
          onPressed: () => showSignIn(context, ref),
          style: FilledButton.styleFrom(
            backgroundColor: palette.accent,
            foregroundColor: Colors.white,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(Radii.small),
            ),
            padding: const EdgeInsets.symmetric(horizontal: Insets.lg, vertical: Insets.md),
          ),
          child: const Text('Sign In'),
        ),
      );
    }

    return _SettingRow(
      title: 'Signed in',
      subtitle: 'Storefront ${session.storefront.toUpperCase()}',
      trailing: TextButton(
        onPressed: () => ref.read(sessionProvider.notifier).signOut(),
        child: Text('Sign Out', style: TextStyles.body.copyWith(color: palette.accent)),
      ),
    );
  }
}

/// The quality tiers, wired to the engine.
///
/// The selection is read from and written to the core, not held in this widget.
/// It previously lived in local `setState`, which meant the choice was discarded
/// the moment the screen was left — a picker that looked like it worked and had
/// no effect, which is worse than one that is obviously disabled.
class _QualityPicker extends ConsumerStatefulWidget {
  const _QualityPicker();

  @override
  ConsumerState<_QualityPicker> createState() => _QualityPickerState();
}

class _QualityPickerState extends ConsumerState<_QualityPicker> {
  /// The tiers, least capable first, which is the order Apple Music uses.
  static const _tiers = <(String, String, String)>[
    ('high', 'High Quality', 'AAC 256 kbps'),
    ('lossless', 'Lossless', 'ALAC up to 24-bit/48 kHz'),
    ('hi_res_lossless', 'Hi-Res Lossless', 'ALAC up to 24-bit/192 kHz'),
    ('spatial', 'Spatial Audio', 'Object-based, rendered binaurally'),
  ];

  /// The active tier's machine name, from the core.
  String? _selected;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      if (!mounted) return;
      // Read once on open, so the picker reflects what the core actually holds
      // rather than defaulting to the first row.
      final engine = ref.read(engineProvider).requireValue;
      final current = await engine.quality();
      if (!mounted) return;
      setState(() => _selected = _tierForLabel(current));
    });
  }

  /// Maps the core's label back to a tier id.
  ///
  /// The core answers with a display label, so the mapping is by label rather
  /// than by re-deriving the tier from the order.
  String _tierForLabel(String label) {
    for (final (id, name, _) in _tiers) {
      if (name == label) return id;
    }
    return 'high';
  }

  Future<void> _select(String id) async {
    final previous = _selected;
    setState(() => _selected = id);
    try {
      final engine = ref.read(engineProvider).requireValue;
      await engine.setQuality(quality: id);
    } on Exception catch (error) {
      if (!mounted) return;
      // Put the selection back, because the core did not take it. A picker that
      // shows one thing while the core holds another is the bug this replaces.
      setState(() => _selected = previous);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not change the quality: $error')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final capabilities = ref.watch(capabilitiesProvider);
    // A tier is usable when the component it depends on is ready. Reading that
    // rather than a separate flag means the picker cannot offer something the
    // machine cannot do.
    final losslessReady = capabilities.maybeWhen(
      data: (list) => list.any((c) => c.id == 'fairplay' && c.ready),
      orElse: () => false,
    );
    final spatialReady = capabilities.maybeWhen(
      data: (list) => list.any((c) => c.id == 'spatial' && c.ready),
      orElse: () => false,
    );
    final losslessState = capabilities.maybeWhen(
      data: (list) => list.where((c) => c.id == 'fairplay').firstOrNull?.state,
      orElse: () => null,
    );
    final spatialState = capabilities.maybeWhen(
      data: (list) => list.where((c) => c.id == 'spatial').firstOrNull?.state,
      orElse: () => null,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (id, label, description) in _tiers)
          _ChoiceRow(
            label: label,
            note: switch (label) {
              'High Quality' => description,
              // An unavailable tier stays visible with the reason, rather than
              // being hidden: knowing the option exists is what tells a person
              // what installing the component would give them.
              'Spatial Audio' => spatialReady
                  ? description
                  : spatialState == 'available'
                      ? 'Needs an object renderer (see Enhancements)'
                      : 'Not available on this machine',
              _ => losslessReady
                  ? description
                  : losslessState == 'available'
                      ? 'Needs the lossless component (see Enhancements)'
                      : 'Not available on this machine',
            },
            enabled: switch (label) {
              'High Quality' => true,
              'Spatial Audio' => spatialReady,
              _ => losslessReady,
            },
            selected: id == _selected,
            onTap: () => _select(id),
          ),
      ],
    );
  }
}

/// The capability rows, one per subsystem.
class _CapabilityList extends ConsumerWidget {
  const _CapabilityList({required this.capabilities});

  final List<CapabilityInfo> capabilities;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final install = ref.watch(installProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (index, capability) in capabilities.indexed) ...[
          if (index > 0) Divider(color: palette.separator, height: Insets.xl),
          _CapabilityEntry(capability: capability, install: install),
        ],
      ],
    );
  }
}

/// One capability, with whatever action applies to it.
class _CapabilityEntry extends ConsumerWidget {
  const _CapabilityEntry({required this.capability, required this.install});

  final CapabilityInfo capability;
  final InstallState install;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    // The lossless tier is the one with an in-app installer, so it is the one
    // that owns the progress row. Everything else is either ready, impossible
    // here, or needs something outside the app.
    final installing = capability.id == 'fairplay' && install.phase != InstallPhase.idle;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _SettingRow(
          title: capability.name,
          subtitle: capability.detail,
          trailing: capability.ready
              ? _StatusBadge(label: 'Ready', colour: palette.blue)
              : _CapabilityAction(capability: capability),
        ),
        if (installing) ...[
          const SizedBox(height: Insets.md),
          InstallProgress(state: install),
        ],
      ],
    );
  }
}

/// The action for a capability that is not ready.
class _CapabilityAction extends ConsumerWidget {
  const _CapabilityAction({required this.capability});

  final CapabilityInfo capability;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final install = ref.watch(installProvider);

    // `unsupported` means the machine cannot run it at all, so no action is
    // offered. Telling someone to install something that cannot work is worse
    // than saying nothing.
    if (capability.isUnsupported) {
      return _StatusBadge(label: 'Unavailable', colour: palette.textTertiary);
    }

    // The lossless tier has a real installer.
    if (capability.id == 'fairplay') {
      if (install.isRunning) {
        return _StatusBadge(label: 'Installing', colour: palette.blue);
      }
      // A failure left the button reading "Try Again" rather than silently
      // reverting, so the second press is obviously the same action.
      final retry = install.phase == InstallPhase.failed;
      return OutlinedButton(
        onPressed: () async {
          ref.read(installProvider.notifier).reset();
          await ref.read(installProvider.notifier).start();
        },
        style: _outlined(palette),
        child: Text(
          retry ? 'Try Again' : (capability.action ?? 'Install'),
          style: TextStyles.body.copyWith(color: palette.textPrimary),
        ),
      );
    }

    // Everything else needs something Sosumi does not ship, and cannot fetch on
    // the user's behalf. The remedy is stated as what it is, and the button says
    // so rather than pretending to act.
    if (capability.action != null) {
      return Tooltip(
        message: '${capability.action} is installed outside Sosumi',
        child: Text(
          capability.action!,
          style: TextStyles.caption1.copyWith(color: palette.textTertiary),
        ),
      );
    }

    return const SizedBox.shrink();
  }

  ButtonStyle _outlined(SosumiPalette palette) => OutlinedButton.styleFrom(
        side: BorderSide(color: palette.glassBorder),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.pill)),
        padding: const EdgeInsets.symmetric(horizontal: Insets.lg),
      );
}

class _CapabilityLoading extends StatelessWidget {
  const _CapabilityLoading();

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      children: [
        SizedBox(
          width: 14,
          height: 14,
          child: CircularProgressIndicator(strokeWidth: 1.5, color: palette.textTertiary),
        ),
        const SizedBox(width: Insets.md),
        Text('Checking what is available\u2026',
            style: TextStyles.body.copyWith(color: palette.textTertiary)),
      ],
    );
  }
}

class _CapabilityError extends StatelessWidget {
  const _CapabilityError({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(Icons.warning_amber_rounded, size: 14, color: palette.accent),
        const SizedBox(width: Insets.sm),
        Expanded(
          child: Text(
            'Could not read the capability list: $message',
            style: TextStyles.caption1.copyWith(color: palette.textSecondary),
          ),
        ),
      ],
    );
  }
}

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.label, required this.colour});

  final String label;
  final Color colour;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: Insets.sm, vertical: 2),
      decoration: BoxDecoration(
        color: colour.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(Radii.pill),
      ),
      child: Text(label, style: TextStyles.caption2.copyWith(color: colour)),
    );
  }
}

class _SettingRow extends StatelessWidget {
  const _SettingRow({required this.title, required this.subtitle, this.trailing});

  final String title;
  final String subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(title, style: TextStyles.body.copyWith(color: palette.textPrimary)),
              const SizedBox(height: 1),
              Text(subtitle, style: TextStyles.caption1.copyWith(color: palette.textSecondary)),
            ],
          ),
        ),
        if (trailing != null) ...[
          const SizedBox(width: Insets.md),
          trailing!,
        ],
      ],
    );
  }
}

class _ChoiceRow extends StatelessWidget {
  const _ChoiceRow({
    required this.label,
    required this.note,
    required this.enabled,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final String? note;
  final bool enabled;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return InteractiveGlass(
      onTap: enabled ? onTap : null,
      radius: Radii.row,
      selected: selected,
      tooltip: enabled ? null : note,
      padding: const EdgeInsets.symmetric(vertical: Insets.sm, horizontal: Insets.sm),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  label,
                  style: (selected ? TextStyles.rowTitle : TextStyles.body).copyWith(
                    color: enabled ? palette.textPrimary : palette.textTertiary,
                  ),
                ),
                if (note != null) ...[
                  const SizedBox(height: 1),
                  Text(note!, style: TextStyles.caption1.copyWith(color: palette.textTertiary)),
                ],
              ],
            ),
          ),
          // The checkmark carries the meaning and the tint reinforces it, so the
          // selection does not depend on distinguishing a colour alone
          // (`accessibility.md`: "Convey information with more than color alone").
          if (selected)
            Icon(Icons.check_rounded, size: 14, color: palette.accent)
          else if (!enabled)
            Icon(Icons.lock_outline_rounded, size: 12, color: palette.textTertiary),
        ],
      ),
    );
  }
}

class _About extends StatelessWidget {
  const _About();

  @override
  Widget build(BuildContext context) {
    return Text(
      'Sosumi is an unofficial client and is not affiliated with Apple. '
      'An active Apple Music subscription is required. See the project '
      'documentation for the full legal scope.',
      style: TextStyles.caption1.copyWith(color: context.palette.textTertiary),
    );
  }
}
