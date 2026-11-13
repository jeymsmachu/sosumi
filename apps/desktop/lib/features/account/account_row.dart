import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/apple_sign_in.dart';
import '../../core/session.dart';
import '../../theme/theme.dart';

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

/// Runs the sign-in flow and reports the outcome in the interface.
///
/// The HIG is explicit that sign-in should be delayed until it is needed and
/// that the request should explain why the account is required. This is called
/// only when the user asks, and the pane below states the reason before handing
/// off to Apple's own page.
///
/// Deliberately not a dialog that never closes: the user may decline, and that
/// must leave the app exactly as it was.
Future<void> showSignIn(BuildContext context, WidgetRef ref) async {
  final messenger = ScaffoldMessenger.maybeOf(context);

  // Explain before opening a web view, so the user knows what they are about to
  // see and why.
  final proceed = await showDialog<bool>(
    context: context,
    barrierColor: Colors.black.withValues(alpha: 0.4),
    builder: (context) => const _SignInExplainer(),
  );
  if (proceed != true) return;

  final result = await AppleSignIn.run();

  switch (result) {
    case SignInSuccess(:final userToken, :final storefront):
      await ref.read(sessionProvider.notifier).setSession(
            musicUserToken: userToken,
            storefront: storefront,
          );
    case SignInCancelled():
      // Nothing to say: the user changed their mind.
      break;
    case SignInUnavailable(:final reason, :final detail):
      // A limitation of the environment, not a mistake by the user, so it is
      // explained rather than reported as an error.
      if (context.mounted) {
        await showDialog<void>(
          context: context,
          builder: (context) => _SignInUnavailable(reason: reason, detail: detail),
        );
      }
    case SignInFailed(:final message):
      messenger?.showSnackBar(
        SnackBar(
          content: Text(message),
          behavior: SnackBarBehavior.floating,
        ),
      );
  }
}

/// Explains an environment that cannot complete sign-in.
///
/// The reason matters. A user who has just typed a correct password and a
/// correct verification code and then sees "Authorization failed" will assume
/// they typed something wrong, and will try again, and again. Saying plainly
/// that the engine cannot do it, and what to use instead, respects their time.
class _SignInUnavailable extends StatelessWidget {
  const _SignInUnavailable({required this.reason, required this.detail});

  final String reason;
  final String detail;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return Dialog(
      backgroundColor: Colors.transparent,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: GlassSurface(
          radius: Radii.panel,
          blur: 44,
          fill: palette.elevatedGlass,
          padding: const EdgeInsets.all(Insets.lg),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Icon(Icons.info_outline_rounded, size: 18, color: palette.textSecondary),
                  const SizedBox(width: Insets.sm),
                  Text('Sign-in is unavailable here',
                      style: TextStyles.title3.copyWith(color: palette.textPrimary)),
                ],
              ),
              const SizedBox(height: Insets.md),
              Text(reason, style: TextStyles.body.copyWith(color: palette.textSecondary)),
              const SizedBox(height: Insets.md),
              Text(
                'Apple\u2019s sign-in page registers a protected-media key system as part '
                'of starting up. WebKitGTK, the engine this app embeds, is built '
                'without that support on most distributions, so the page cannot '
                'finish. This is an engine limitation, not a problem with your '
                'account or your password.',
                style: TextStyles.caption1.copyWith(color: palette.textTertiary),
              ),
              const SizedBox(height: Insets.lg),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  FilledButton(
                    onPressed: () => Navigator.of(context).pop(),
                    style: FilledButton.styleFrom(
                      backgroundColor: palette.accent,
                      foregroundColor: Colors.white,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(Radii.small),
                      ),
                      padding: const EdgeInsets.symmetric(
                        horizontal: Insets.lg,
                        vertical: Insets.md,
                      ),
                    ),
                    child: const Text('OK'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// What signing in provides, and what Sosumi does with it.
///
/// Apple's guidance is to write a brief, friendly description of the reason for
/// the requirement and its benefit, so this states both plainly and names the
/// one thing a cautious user actually wants to know: where the password goes.
class _SignInExplainer extends StatelessWidget {
  const _SignInExplainer();

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return Dialog(
      backgroundColor: Colors.transparent,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 400),
        child: GlassSurface(
          radius: Radii.panel,
          blur: 44,
          fill: palette.elevatedGlass,
          padding: const EdgeInsets.all(Insets.lg),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Icon(Icons.apple, size: 20, color: palette.textPrimary),
                  const SizedBox(width: Insets.sm),
                  Text('Sign in to Apple Music',
                      style: TextStyles.title3.copyWith(color: palette.textPrimary)),
                ],
              ),
              const SizedBox(height: Insets.md),
              Text(
                'Sosumi plays music from your Apple Music subscription. Signing in '
                'connects the account so the app can stream and download what your '
                'subscription covers.',
                style: TextStyles.body.copyWith(color: palette.textSecondary),
              ),
              const SizedBox(height: Insets.md),
              // The reassurance worth stating: the password stays with Apple.
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(Icons.lock_outline_rounded, size: 14, color: palette.textTertiary),
                  const SizedBox(width: Insets.sm),
                  Expanded(
                    child: Text(
                      'You sign in on Apple\u2019s own page, inside this window. Your '
                      'Apple ID password goes directly to Apple and is never seen or '
                      'stored by Sosumi.',
                      style: TextStyles.caption1.copyWith(color: palette.textTertiary),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Insets.lg),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(false),
                    child: Text('Not Now',
                        style: TextStyles.body.copyWith(color: palette.textSecondary)),
                  ),
                  const SizedBox(width: Insets.sm),
                  FilledButton(
                    onPressed: () => Navigator.of(context).pop(true),
                    style: FilledButton.styleFrom(
                      backgroundColor: palette.accent,
                      foregroundColor: Colors.white,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(Radii.small),
                      ),
                      padding: const EdgeInsets.symmetric(
                        horizontal: Insets.lg,
                        vertical: Insets.md,
                      ),
                    ),
                    child: const Text('Continue'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
