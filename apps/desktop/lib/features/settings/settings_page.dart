import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/capabilities.dart';
import '../../core/session.dart';
import '../../theme/theme.dart';

/// The Settings page.
///
/// It leads with the account, then the sound-quality choice, then the optional
/// capabilities. That order matches what a person comes here for: who they are,
/// what they hear, and what else they could have.
///
/// Capability rows read live state rather than shipping a hard-coded list, so the
/// screen cannot claim something is installed when it is not. See
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
        const _Group(
          title: 'Account',
          children: [_AccountRow()],
        ),
        const SizedBox(height: Insets.lg),
        const _Group(
          title: 'Audio Quality',
          children: [_QualityRow()],
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
        _About(signedIn: session.signedIn),
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
            title.toUpperCase(),
            style: TextStyles.caption2.copyWith(color: palette.textSecondary, letterSpacing: 0.6),
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
  const _AccountRow();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final palette = context.palette;

    return _SettingRow(
      title: session.signedIn ? 'Signed in' : 'Not Signed In',
      subtitle: session.signedIn
          ? 'Storefront ${session.storefront.toUpperCase()}'
          : 'An active Apple Music subscription is required',
      trailing: session.signedIn
          ? TextButton(
              onPressed: () => ref.read(sessionProvider.notifier).signOut(),
              child: Text('Sign Out',
                  style: TextStyles.body.copyWith(color: palette.accent)),
            )
          : null,
    );
  }
}

class _QualityRow extends ConsumerStatefulWidget {
  const _QualityRow();

  @override
  ConsumerState<_QualityRow> createState() => _QualityRowState();
}

class _QualityRowState extends ConsumerState<_QualityRow> {
  /// The tiers, least capable first, which is the order Apple Music uses.
  static const _tiers = <(String, String)>[
    ('High Quality', 'AAC 256 kbps'),
    ('Lossless', 'ALAC up to 24-bit/48 kHz'),
    ('Hi-Res Lossless', 'ALAC up to 24-bit/192 kHz'),
    ('Spatial Audio', 'Object-based, rendered binaurally'),
  ];

  String _selected = 'High Quality';

  @override
  Widget build(BuildContext context) {
    final capabilities = ref.watch(capabilitiesProvider);
    // A tier is usable when the capability it depends on is ready. Reading that
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

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (label, description) in _tiers)
          _ChoiceRow(
            label: label,
            note: switch (label) {
              'High Quality' => description,
              // An unavailable tier stays visible with the reason, rather than
              // being hidden: knowing the option exists is what tells a person
              // what installing the component would give them.
              'Spatial Audio' =>
                spatialReady ? description : 'Requires an object-based renderer',
              _ => losslessReady ? description : 'Requires the Lossless component',
            },
            enabled: switch (label) {
              'High Quality' => true,
              'Spatial Audio' => spatialReady,
              _ => losslessReady,
            },
            selected: label == _selected,
            onTap: () => setState(() => _selected = label),
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

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (index, capability) in capabilities.indexed) ...[
          if (index > 0) Divider(color: palette.separator, height: Insets.xl),
          _SettingRow(
            title: capability.name,
            subtitle: capability.detail,
            trailing: capability.ready
                ? _StatusBadge(label: 'Ready', colour: palette.blue)
                : _InstallButton(capability: capability),
          ),
        ],
      ],
    );
  }
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

/// The install action for a capability that is not ready.
class _InstallButton extends ConsumerStatefulWidget {
  const _InstallButton({required this.capability});

  final CapabilityInfo capability;

  @override
  ConsumerState<_InstallButton> createState() => _InstallButtonState();
}

class _InstallButtonState extends ConsumerState<_InstallButton> {
  bool _busy = false;

  @override
  Widget build(BuildContext context) {
    // `unsupported` means the machine cannot run it at all, so no action is
    // offered. Telling someone to install something that cannot work is worse
    // than saying nothing.
    if (widget.capability.state == 'unsupported') {
      return const SizedBox.shrink();
    }

    // An action that needs the user to do something outside the app, such as
    // starting Discord, is shown as disabled text rather than a button that
    // cannot do anything.
    if (widget.capability.action != null && widget.capability.missing.isEmpty) {
      return Text(
        widget.capability.action!,
        style: TextStyles.caption1.copyWith(color: context.palette.textTertiary),
      );
    }

    return OutlinedButton(
      onPressed: _busy ? null : _install,
      style: OutlinedButton.styleFrom(
        side: BorderSide(color: context.palette.glassBorder),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.pill)),
        padding: const EdgeInsets.symmetric(horizontal: Insets.lg),
      ),
      child: _busy
          ? const SizedBox(
              width: 12,
              height: 12,
              child: CircularProgressIndicator(strokeWidth: 1.5),
            )
          : Text(widget.capability.action ?? 'Install',
              style: TextStyles.body.copyWith(color: context.palette.textPrimary)),
    );
  }

  Future<void> _install() async {
    setState(() => _busy = true);
    await ref.read(capabilitiesProvider.notifier).install(widget.capability.id);
    if (mounted) setState(() => _busy = false);
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
          // selection does not depend on distinguishing a colour.
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
  const _About({required this.signedIn});

  final bool signedIn;

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
