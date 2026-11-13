import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/capabilities.dart';
import '../../core/session.dart';
import '../../theme/theme.dart';
import 'sign_in_sheet.dart';

/// The account control at the foot of the sidebar.
///
/// Apple's shape: a circular avatar on the leading side and the display name
/// beside it. Selecting it does **not** open a profile page; it reveals a menu of
/// account commands, which is what the reference app does and what the HIG asks
/// for — a menu is the space-efficient way to present a short list of commands.
///
/// The menu follows the HIG's rules: title-style capitalization, no articles,
/// icons only where they clarify (and uniformly within a group), a separator
/// between the account commands and the session command, the accent reserved for
/// the destructive one, and unavailable items shown dimmed rather than omitted.
class AccountRow extends ConsumerWidget {
  const AccountRow({super.key, required this.collapsed, required this.onOpenSettings});

  final bool collapsed;

  /// Opens the Settings section. The shell owns navigation, so the row reports
  /// the intent rather than reaching into the destination list.
  final VoidCallback onOpenSettings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final palette = context.palette;

    return PopupMenuButton<AccountAction>(
      // Opens upward, because the row sits at the foot of the window where there
      // is no room beneath it.
      position: PopupMenuPosition.over,
      offset: const Offset(0, -8),
      tooltip: '',
      color: palette.elevatedGlass,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.row),
        side: BorderSide(color: palette.glassBorder, width: 0.5),
      ),
      onSelected: (action) => _handle(context, ref, action),
      itemBuilder: (context) => _menuItems(context, session.signedIn),
      child: _AccountLabel(session: session, collapsed: collapsed, palette: palette),
    );
  }

  List<PopupMenuEntry<AccountAction>> _menuItems(BuildContext context, bool signedIn) {
    final palette = context.palette;
    return [
      // Help and Transfer Music have no implementation yet. They are shown
      // disabled rather than omitted: the HIG asks for an unavailable item to
      // stay visible and dimmed so a menu's contents stay predictable.
      _item(AccountAction.help, 'Help', Icons.help_outline_rounded, palette, enabled: false),
      _item(AccountAction.settings, 'Settings…', Icons.settings_outlined, palette),
      _item(
        AccountAction.transferMusic,
        'Transfer Music…',
        Icons.arrow_right_alt_rounded,
        palette,
        enabled: false,
      ),
      const PopupMenuDivider(),
      if (signedIn)
        _item(AccountAction.signOut, 'Sign Out', null, palette)
      else
        _item(AccountAction.signIn, 'Sign In…', null, palette),
    ];
  }

  /// A menu item. Icons are given to every item in the group or to none, which
  /// is the HIG's rule for a uniform treatment.
  PopupMenuItem<AccountAction> _item(
    AccountAction action,
    String label,
    IconData? icon,
    SosumiPalette palette, {
    bool enabled = true,
  }) {
    final color = !enabled
        ? palette.textTertiary
        : action == AccountAction.signOut
            ? palette.accent
            : palette.textPrimary;

    return PopupMenuItem<AccountAction>(
      value: action,
      enabled: enabled,
      height: 32,
      child: Row(
        children: [
          Expanded(child: Text(label, style: TextStyles.body.copyWith(color: color))),
          if (icon != null) ...[
            const SizedBox(width: Insets.xl),
            Icon(icon, size: 14, color: color),
          ],
        ],
      ),
    );
  }

  void _handle(BuildContext context, WidgetRef ref, AccountAction action) {
    switch (action) {
      case AccountAction.signIn:
        unawaited(showSignIn(context, ref));
      case AccountAction.signOut:
        unawaited(ref.read(sessionProvider.notifier).signOut());
      case AccountAction.settings:
        onOpenSettings();
      case AccountAction.help:
      case AccountAction.transferMusic:
        // Disabled in the menu, so this is unreachable.
        break;
    }
  }
}

/// The commands the account menu offers.
enum AccountAction { help, settings, transferMusic, signIn, signOut }

class _AccountLabel extends StatelessWidget {
  const _AccountLabel({
    required this.session,
    required this.collapsed,
    required this.palette,
  });

  final SessionState session;
  final bool collapsed;
  final SosumiPalette palette;

  @override
  Widget build(BuildContext context) {
    final name = session.signedIn ? (session.displayName ?? 'Account') : 'Sign In';

    return Padding(
      padding: const EdgeInsets.all(Insets.sm),
      child: SizedBox(
        height: Sizes.sidebarRowHeight,
        child: Row(
          children: [
            _Avatar(session: session, palette: palette),
            if (!collapsed) ...[
              const SizedBox(width: Insets.sm),
              Expanded(
                child: Text(
                  name,
                  style: TextStyles.body.copyWith(color: palette.textPrimary),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Icon(Icons.unfold_more_rounded, size: 13, color: palette.textTertiary),
            ],
          ],
        ),
      ),
    );
  }
}

/// The circular avatar. Signed out it is a neutral person glyph; signed in it is
/// the account's artwork.
class _Avatar extends StatelessWidget {
  const _Avatar({required this.session, required this.palette});

  final SessionState session;
  final SosumiPalette palette;

  @override
  Widget build(BuildContext context) {
    final url = session.avatarUrl;
    return ClipOval(
      child: SizedBox(
        width: 24,
        height: 24,
        child: url == null
            ? ColoredBox(
                color: palette.accent,
                child: const Icon(Icons.person_rounded, size: 15, color: Colors.white),
              )
            : Image.network(
                url,
                fit: BoxFit.cover,
                errorBuilder: (_, _, _) => ColoredBox(
                  color: palette.accent,
                  child: const Icon(Icons.person_rounded, size: 15, color: Colors.white),
                ),
              ),
      ),
    );
  }
}

/// Runs the native sign-in flow.
///
/// Deliberately not a web view. Apple's sign-in page cannot complete on Linux
/// because WebKitGTK ships without Encrypted Media, so the page reports an
/// authorization error after a correct password — see `docs/PLATFORM-LIMITS.md`.
/// This drives Apple's own account libraries instead, which need no browser.
///
/// The HIG is explicit that sign-in should be delayed until it is needed and that
/// the request should say why the account is required, so the sheet opens only
/// when asked and states the reason before collecting anything.
Future<void> showSignIn(BuildContext context, WidgetRef ref) async {
  final succeeded = await SignInSheet.show(context);
  if (!succeeded || !context.mounted) return;

  // Adopted, not merely refreshed: the native flow wrote the token into its own
  // store, and reading the app's session back without adopting would report an
  // unauthenticated session even though the sign-in just succeeded.
  //
  // Reported rather than thrown: the sign-in did succeed, and an unhandled
  // exception out of a void callback would read as a crash. Apple's own reason
  // is surfaced when there is one.
  try {
    await ref.read(sessionProvider.notifier).adoptNativeSession();
  } on Exception catch (error) {
    if (!context.mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Signed in, but the session could not be loaded: $error')),
    );
    return;
  }
  if (!context.mounted) return;
  ref.invalidate(capabilitiesProvider);
}
