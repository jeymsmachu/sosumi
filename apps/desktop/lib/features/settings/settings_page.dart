import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/session.dart';
import '../../theme/theme.dart';

/// The Settings page.
///
/// It leads with the sound-quality choice, then sign-in, then the optional
/// capabilities. The order matters: quality is what a user comes here for, and
/// capabilities explain what a higher tier would require.
class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);

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
          children: [
            _AccountRow(),
          ],
        ),
        const SizedBox(height: Insets.lg),
        const _Group(
          title: 'Audio quality',
          children: [
            _QualityRow(),
          ],
        ),
        const SizedBox(height: Insets.lg),
        const _Group(
          title: 'Enhancements',
          children: [
            _CapabilityRow(),
          ],
        ),
        const SizedBox(height: Insets.xl),
        _About(signedIn: session.signedIn),
      ],
    );
  }
}

/// A titled group of rows, drawn as one glass panel.
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
          padding: const EdgeInsets.only(left: Insets.xs, bottom: Insets.sm),
          child: Text(
            title.toUpperCase(),
            style: TextStyles.caption1.copyWith(color: palette.textSecondary),
          ),
        ),
        // Content, not chrome. Settings groups are content-layer cards, so they
        // use a standard surface; glass belongs only on the sidebar and the
        // transport. See materials.md: "Don't use Liquid Glass in the content
        // layer."
        ContentSurface(
          radius: Radii.large,
          elevated: true,
          padding: const EdgeInsets.symmetric(
            horizontal: Insets.lg,
            vertical: Insets.md,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: children,
          ),
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
      title: session.signedIn ? 'Signed in' : 'Not signed in',
      subtitle: session.signedIn
          ? 'Storefront ${session.storefront.toUpperCase()}'
          : 'Sign in with your Apple Music subscription',
      trailing: session.signedIn
          ? TextButton(
              onPressed: () => ref.read(sessionProvider.notifier).signOut(),
              child: Text(
                'Sign Out',
                style: TextStyles.headline.copyWith(color: palette.accent),
              ),
            )
          : FilledButton(
              onPressed: () {
                // The web sign-in flow is wired once the login view lands.
              },
              child: const Text('Sign In'),
            ),
    );
  }
}

class _QualityRow extends ConsumerStatefulWidget {
  const _QualityRow();

  @override
  ConsumerState<_QualityRow> createState() => _QualityRowState();
}

class _QualityRowState extends ConsumerState<_QualityRow> {
  /// The tiers, most capable last, matching Apple Music's own ordering.
  static const _tiers = <(String, String, String?)>[
    ('high', 'High Quality', null),
    ('lossless', 'Lossless', 'Requires the FairPlay component'),
    ('hi_res_lossless', 'Hi-Res Lossless', 'Requires the FairPlay component'),
    ('spatial', 'Spatial Audio', 'Requires the object-based renderer'),
  ];

  String _selected = 'high';

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (value, label, note) in _tiers)
          _ChoiceRow(
            label: label,
            note: note,
            // A tier needing a component that is not installed stays visible but
            // is not selectable: showing the requirement is better than hiding
            // the option, and much better than failing after the choice.
            enabled: note == null,
            selected: value == _selected,
            onTap: () => setState(() => _selected = value),
          ),
      ],
    );
  }
}

class _CapabilityRow extends StatelessWidget {
  const _CapabilityRow();

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _SettingRow(
          title: 'Lossless & Hi-Res',
          subtitle: 'Not installed. Enables ALAC up to 24-bit/192 kHz.',
          trailing: _InstallButton(onPressed: () {}),
        ),
        Divider(color: palette.separator, height: Insets.xl),
        _SettingRow(
          title: 'Spatial Audio',
          subtitle: 'Not installed. Renders object-based audio binaurally.',
          trailing: _InstallButton(onPressed: () {}),
        ),
        Divider(color: palette.separator, height: Insets.xl),
        const _SettingRow(
          title: 'System Media Controls',
          subtitle: 'Available. Control playback from your desktop.',
          trailing: _Toggle(value: true),
        ),
      ],
    );
  }
}

class _SettingRow extends StatelessWidget {
  const _SettingRow({
    required this.title,
    required this.subtitle,
    this.trailing,
  });

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
              Text(title, style: TextStyles.rowTitle.copyWith(color: palette.textPrimary)),
              const SizedBox(height: 2),
              Text(
                subtitle,
                style: TextStyles.callout.copyWith(color: palette.textSecondary),
              ),
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
      padding: const EdgeInsets.symmetric(
        vertical: Insets.sm + 2,
        horizontal: Insets.sm,
      ),
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
                  const SizedBox(height: 2),
                  Text(
                    note!,
                    style: TextStyles.caption1.copyWith(color: palette.textTertiary),
                  ),
                ],
              ],
            ),
          ),
          // The selection indicator. A colour change alone would fail for anyone
          // who cannot distinguish the accent, so the checkmark carries the
          // meaning and the tint only reinforces it.
          if (selected)
            Icon(Icons.check_rounded, size: 15, color: palette.accent)
          else if (!enabled)
            Icon(Icons.lock_outline_rounded, size: 13, color: palette.textTertiary),
        ],
      ),
    );
  }
}

class _InstallButton extends StatelessWidget {
  const _InstallButton({required this.onPressed});

  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return OutlinedButton(
      onPressed: onPressed,
      style: OutlinedButton.styleFrom(
        side: BorderSide(color: context.palette.glassBorder),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.pill)),
        padding: const EdgeInsets.symmetric(horizontal: Insets.lg),
      ),
      child: Text('Install', style: TextStyles.headline.copyWith(color: context.palette.textPrimary)),
    );
  }
}

class _Toggle extends StatelessWidget {
  const _Toggle({required this.value});

  final bool value;

  @override
  Widget build(BuildContext context) {
    return Switch(value: value, onChanged: (_) {});
  }
}

class _About extends StatelessWidget {
  const _About({required this.signedIn});

  final bool signedIn;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Sosumi is an unofficial client and is not affiliated with Apple. '
          'An active Apple Music subscription is required. See the project '
          'documentation for the full legal scope.',
          style: TextStyles.caption1.copyWith(color: palette.textSecondary),
        ),
      ],
    );
  }
}
