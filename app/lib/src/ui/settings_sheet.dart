import 'package:flutter/material.dart';

import '../state/sosumi_controller.dart';
import 'glass_surface.dart';

/// Opens the appearance settings sheet.
Future<void> showSettingsSheet(BuildContext context) {
  final controller = SosumiScope.of(context);
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: Colors.transparent,
    barrierColor: Colors.black.withValues(alpha: 0.55),
    builder: (context) {
      return Padding(
        padding: EdgeInsets.only(
          left: 16,
          right: 16,
          bottom: 16 + MediaQuery.of(context).viewInsets.bottom,
        ),
        child: Align(
          alignment: Alignment.bottomCenter,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 620),
            child: GlassSurface(
              enabled: controller.glassEnabled,
              radius: 30,
              padding: const EdgeInsets.fromLTRB(22, 18, 22, 22),
              child: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      children: [
                        Text(
                          'Settings',
                          style: Theme.of(context).textTheme.headlineSmall,
                        ),
                        const Spacer(),
                        IconButton(
                          tooltip: 'Close settings',
                          onPressed: () => Navigator.of(context).pop(),
                          icon: const Icon(Icons.close_rounded),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      'Make the room sound like yours.',
                      style: Theme.of(context).textTheme.bodyMedium,
                    ),
                    const SizedBox(height: 18),
                    const _SectionLabel('Appearance'),
                    SwitchListTile.adaptive(
                      contentPadding: EdgeInsets.zero,
                      secondary: const Icon(Icons.blur_on_rounded),
                      title: const Text('Glass effects'),
                      subtitle: const Text(
                        'Backdrop blur and translucent chrome.',
                      ),
                      value: controller.glassEnabled,
                      onChanged: controller.setGlassEnabled,
                    ),
                    SwitchListTile.adaptive(
                      contentPadding: EdgeInsets.zero,
                      secondary: const Icon(Icons.motion_photos_on_rounded),
                      title: const Text('Ambient motion'),
                      subtitle: const Text(
                        'Slow background drift and transitions.',
                      ),
                      value: controller.motionEnabled,
                      onChanged: controller.setMotionEnabled,
                    ),
                    SwitchListTile.adaptive(
                      contentPadding: EdgeInsets.zero,
                      secondary: const Icon(Icons.density_medium_rounded),
                      title: const Text('Compact density'),
                      subtitle: const Text(
                        'Tighter spacing for smaller windows.',
                      ),
                      value: controller.compactDensity,
                      onChanged: controller.setCompactDensity,
                    ),
                    const SizedBox(height: 12),
                    const _SectionLabel('Accent'),
                    const SizedBox(height: 10),
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
                    const SizedBox(height: 22),
                    const Divider(),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        const Icon(Icons.info_outline_rounded, size: 18),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'sosumi 0.1.0 · scaffold build. Playback adapters are not connected yet.',
                            style: Theme.of(context).textTheme.bodySmall,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
    },
  );
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    return Text(
      label.toUpperCase(),
      style: Theme.of(context).textTheme.labelSmall?.copyWith(
        color: Colors.white.withValues(alpha: 0.46),
        letterSpacing: 1.4,
        fontWeight: FontWeight.w700,
      ),
    );
  }
}
