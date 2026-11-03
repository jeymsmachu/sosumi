import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';

/// Opens the full-screen Settings surface.
Future<void> showSettingsPage(BuildContext context) {
  return Navigator.of(
    context,
    rootNavigator: true,
  ).push(MaterialPageRoute<void>(builder: (_) => const SettingsPage()));
}

/// Full-screen settings, modeled on Apple Music's grouped settings layout.
class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  bool _addDeletePlaylistSongs = false;
  bool _addDeleteFavoriteSongs = false;
  bool _spatialAudio = true;
  bool _soundCheck = true;

  @override
  Widget build(BuildContext context) {
    final controller = SosumiScope.of(context);
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.black,
      body: Material(
        color: Colors.black,
        child: SafeArea(
          bottom: false,
          child: Column(
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(8, 8, 8, 8),
                child: SizedBox(
                  height: 48,
                  child: Stack(
                    alignment: Alignment.center,
                    children: [
                      Align(
                        alignment: Alignment.centerLeft,
                        child: IconButton(
                          tooltip: 'Back',
                          onPressed: () => Navigator.of(context).maybePop(),
                          icon: const Icon(Icons.arrow_back_rounded),
                        ),
                      ),
                      Text('Settings', style: theme.textTheme.titleLarge),
                    ],
                  ),
                ),
              ),
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.fromLTRB(20, 8, 20, 36),
                  children: [
                    const _SectionLabel('Library'),
                    _SettingsCard(
                      children: [
                        _SettingsTile(
                          title: 'Add & Delete Playlist Songs',
                          subtitle: 'Songs added to playlists you create will also be added to your library. Deleting songs from your library will also remove them from those playlists.',
                          trailing: Switch(
                            value: _addDeletePlaylistSongs,
                            onChanged: (value) =>
                                setState(() => _addDeletePlaylistSongs = value),
                          ),
                        ),
                        _SettingsTile(
                          title: 'Add & Delete Favorite Songs',
                          subtitle: 'Songs you favorite will also be added to your library. Deleting songs from your library will also remove them from your favorites.',
                          trailing: Switch(
                            value: _addDeleteFavoriteSongs,
                            onChanged: (value) =>
                                setState(() => _addDeleteFavoriteSongs = value),
                          ),
                        ),
                        const _SettingsTile(
                          title: 'Transfer Music from Other Services',
                          subtitle: 'Add music and playlists saved in other music services to your library.',
                        ),
                      ],
                    ),
                    const SizedBox(height: 28),
                    const _SectionLabel('Audio'),
                    _SettingsCard(
                      children: [
                        _SettingsTile(
                          title: 'Spatial Audio',
                          trailing: Switch(
                            value: _spatialAudio,
                            onChanged: (value) =>
                                setState(() => _spatialAudio = value),
                          ),
                        ),
                        const _SettingsTile(
                          title: 'Audio Quality',
                          subtitle: 'Hi-Res Lossless',
                        ),
                        const _SettingsTile(
                          title: 'Crossfade',
                          subtitle: 'Automatic',
                        ),
                        const _SettingsTile(
                          title: 'Equalizer',
                          subtitle: 'Adjust audio output settings.',
                        ),
                        _SettingsTile(
                          title: 'Sound Check',
                          trailing: Switch(
                            value: _soundCheck,
                            onChanged: (value) =>
                                setState(() => _soundCheck = value),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 28),
                    const _SectionLabel('Appearance'),
                    _SettingsCard(
                      children: [
                        _SettingsTile(
                          title: 'Glass effects',
                          subtitle: 'Backdrop blur and translucent chrome.',
                          trailing: Switch(
                            value: controller.glassEnabled,
                            onChanged: controller.setGlassEnabled,
                          ),
                        ),
                        _SettingsTile(
                          title: 'Ambient motion',
                          subtitle: 'Slow background drift and transitions.',
                          trailing: Switch(
                            value: controller.motionEnabled,
                            onChanged: controller.setMotionEnabled,
                          ),
                        ),
                        _SettingsTile(
                          title: 'Compact density',
                          subtitle: 'Tighter spacing for smaller windows.',
                          trailing: Switch(
                            value: controller.compactDensity,
                            onChanged: controller.setCompactDensity,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 18),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: [
                        for (final choice in accentChoices)
                          ChoiceChip(
                            avatar: CircleAvatar(
                              radius: 6,
                              backgroundColor: choice.color,
                            ),
                            label: Text(choice.label),
                            selected: controller.accent == choice.color,
                            selectedColor: choice.color.withValues(alpha: 0.18),
                            side: BorderSide(
                              color: controller.accent == choice.color
                                  ? choice.color.withValues(alpha: 0.7)
                                  : Colors.white.withValues(alpha: 0.08),
                            ),
                            onSelected: (_) =>
                                controller.setAccent(choice.color),
                          ),
                      ],
                    ),
                    const SizedBox(height: 28),
                    const _SectionLabel('Account'),
                    const _SettingsCard(
                      children: [
                        _SettingsTile(
                          title: 'Apple Music',
                          subtitle: 'Not signed in',
                        ),
                        _SettingsTile(
                          title: 'DRM session',
                          subtitle: 'Not signed in',
                        ),
                      ],
                    ),
                    const SizedBox(height: 28),
                    Text(
                      'sosumi 0.2.0',
                      style: theme.textTheme.bodySmall,
                      textAlign: TextAlign.center,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 4, bottom: 8),
      child: Text(
        label.toUpperCase(),
        style: Theme.of(context).textTheme.labelMedium?.copyWith(
          color: Colors.white.withValues(alpha: 0.52),
          letterSpacing: 1.1,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

class _SettingsCard extends StatelessWidget {
  const _SettingsCard({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(18),
      child: Container(
        color: const Color(0xFF1C1C1E),
        child: Column(
          children: [
            for (var i = 0; i < children.length; i++) ...[
              children[i],
              if (i != children.length - 1)
                Divider(
                  height: 1,
                  thickness: 0.6,
                  indent: 16,
                  color: Colors.white.withValues(alpha: 0.10),
                ),
            ],
          ],
        ),
      ),
    );
  }
}

class _SettingsTile extends StatelessWidget {
  const _SettingsTile({required this.title, this.subtitle, this.trailing});

  final String title;
  final String? subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: const Color(0xFF1C1C1E),
      child: InkWell(
        onTap: trailing == null ? () {} : null,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 14, 12, 14),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: theme.textTheme.titleMedium),
                    if (subtitle != null) ...[
                      const SizedBox(height: 5),
                      Text(subtitle!, style: theme.textTheme.bodyMedium),
                    ],
                  ],
                ),
              ),
              if (trailing != null) ...[
                const SizedBox(width: 12),
                trailing!,
              ] else ...[
                const SizedBox(width: 8),
                Icon(
                  CupertinoIcons.chevron_forward,
                  size: 16,
                  color: Colors.white.withValues(alpha: 0.38),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
