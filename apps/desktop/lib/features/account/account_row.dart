import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/session.dart';
import '../../theme/theme.dart';

/// The account control at the foot of the sidebar.
///
/// Apple's shape: a circular avatar on the leading side and the display name
/// beside it. Selecting it does **not** open a profile page; it reveals a menu of
/// account commands, which is what the reference does and what the HIG asks for
/// — a menu is the space-efficient way to present a short list of commands.
///
/// The menu follows the HIG's rules for menus: title-style capitalization,
/// no articles, icons only where they clarify, a separator between the account
/// commands and signing out, and destructive styling on Sign Out.
class AccountRow extends ConsumerWidget {
  const AccountRow({super.key, required this.collapsed});

  final bool collapsed;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final palette = context.palette;

    return PopupMenuButton<AccountAction>(
      // Anchored to the leading edge and opening upward, because the row sits at
      // the foot of the window where there is no room below.
      position: PopupMenuPosition.over,
      offset: const Offset(0, -8),
      tooltip: '',
      onSelected: (action) => _handle(context, ref, action),
      itemBuilder: (context) => _menuItems(context),
      child: _AccountLabel(session: session, collapsed: collapsed, palette: palette),
    );
  }

  List<PopupMenuEntry<AccountAction>> _menuItems(BuildContext context) {
    final palette = context.palette;
    return [
      _item(AccountAction.help, 'Help', Icons.help_outline_rounded, palette),
      _item(AccountAction.settings, 'Settings…', Icons.settings_outlined, palette),
      _item(
        AccountAction.transferMusic,
        'Transfer Music…',
        Icons.arrow_right_alt_rounded,
        palette,
      ),
      const PopupMenuDivider(),
      if (sessionSignedIn(context))
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
    SosumiPalette palette,
  ) {
    return PopupMenuItem<AccountAction>(
      value: action,
      height: 32,
      child: Row(
        children: [
          Expanded(
            child: Text(
              label,
              style: TextStyles.body.copyWith(
                color: action == AccountAction.signOut ? palette.accent : palette.textPrimary,
              ),
            ),
          ),
          if (icon != null) ...[
            const SizedBox(width: Insets.xl),
            Icon(icon, size: 14, color: palette.textSecondary),
          ],
        ],
      ),
    );
  }

  bool sessionSignedIn(BuildContext context) =>
      ProviderScope.containerOf(context).read(sessionProvider).signedIn;

  void _handle(BuildContext context, WidgetRef ref, AccountAction action) {
    switch (action) {
      case AccountAction.signIn:
        showSignInSheet(context, ref);
      case AccountAction.signOut:
        ref.read(sessionProvider.notifier).signOut();
      case AccountAction.settings:
      case AccountAction.help:
      case AccountAction.transferMusic:
        // Not implemented yet. The menu keeps the item visible and dimmed
        // behaviour arrives with the actions; leaving them out would make the
        // menu's contents unpredictable.
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

/// The circular avatar. Signed out, it is a neutral person glyph; signed in, it
/// is the account's artwork.
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

/// The sign-in sheet.
///
/// The HIG is explicit that sign-in should be delayed until it is needed and
/// that the view must explain why the account is required. This does both: it
/// opens only when asked, and it states what signing in provides and what
/// Sosumi does with the credentials.
void showSignInSheet(BuildContext context, WidgetRef ref) {
  showDialog<void>(
    context: context,
    barrierColor: Colors.black.withValues(alpha: 0.4),
    builder: (context) => const _SignInSheet(),
  );
}

class _SignInSheet extends ConsumerStatefulWidget {
  const _SignInSheet();

  @override
  ConsumerState<_SignInSheet> createState() => _SignInSheetState();
}

class _SignInSheetState extends ConsumerState<_SignInSheet> {
  final _tokenController = TextEditingController();
  String? _error;
  bool _busy = false;

  @override
  void dispose() {
    _tokenController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;

    return Dialog(
      backgroundColor: Colors.transparent,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 380),
        child: GlassSurface(
          radius: Radii.panel,
          blur: 44,
          fill: palette.elevatedGlass,
          padding: const EdgeInsets.all(Insets.lg),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text('Sign In', style: TextStyles.title2.copyWith(color: palette.textPrimary)),
              const SizedBox(height: Insets.sm),
              Text(
                'Sosumi plays music from your Apple Music subscription. Signing in '
                'connects your account so the app can stream and download what your '
                'subscription covers.',
                style: TextStyles.callout.copyWith(color: palette.textSecondary),
              ),
              const SizedBox(height: Insets.lg),
              Text(
                'Your credentials go only to Apple. Sosumi stores the session token '
                'on this machine, encrypted, and never sends it anywhere else.',
                style: TextStyles.caption1.copyWith(color: palette.textTertiary),
              ),
              const SizedBox(height: Insets.lg),
              _Field(
                controller: _tokenController,
                hint: 'Music-User-Token',
                enabled: !_busy,
              ),
              if (_error != null) ...[
                const SizedBox(height: Insets.sm),
                Row(
                  children: [
                    Icon(Icons.warning_amber_rounded, size: 13, color: palette.accent),
                    const SizedBox(width: Insets.xs),
                    Expanded(
                      child: Text(
                        _error!,
                        style: TextStyles.caption1.copyWith(color: palette.accent),
                      ),
                    ),
                  ],
                ),
              ],
              const SizedBox(height: Insets.lg),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: _busy ? null : () => Navigator.of(context).pop(),
                    child: Text('Cancel',
                        style: TextStyles.body.copyWith(color: palette.textSecondary)),
                  ),
                  const SizedBox(width: Insets.sm),
                  FilledButton(
                    onPressed: _busy ? null : _submit,
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
                    child: _busy
                        ? const SizedBox(
                            width: 14,
                            height: 14,
                            child: CircularProgressIndicator(
                              strokeWidth: 2,
                              color: Colors.white,
                            ),
                          )
                        : const Text('Sign In'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _submit() async {
    final token = _tokenController.text.trim();
    if (token.isEmpty) {
      setState(() => _error = 'Paste your Music-User-Token to continue.');
      return;
    }

    setState(() {
      _busy = true;
      _error = null;
    });

    try {
      await ref.read(sessionProvider.notifier).setSession(
            musicUserToken: token,
            storefront: 'us',
          );
      if (mounted) Navigator.of(context).pop();
    } on Exception catch (error) {
      if (mounted) {
        setState(() {
          _busy = false;
          _error = '$error';
        });
      }
    }
  }
}

class _Field extends StatelessWidget {
  const _Field({required this.controller, required this.hint, required this.enabled});

  final TextEditingController controller;
  final String hint;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Container(
      decoration: BoxDecoration(
        color: palette.fill,
        borderRadius: BorderRadius.circular(Radii.small),
        border: Border.all(color: palette.separator, width: 0.5),
      ),
      padding: const EdgeInsets.symmetric(horizontal: Insets.md),
      child: TextField(
        controller: controller,
        enabled: enabled,
        style: TextStyles.body.copyWith(color: palette.textPrimary),
        cursorColor: palette.accent,
        decoration: InputDecoration(
          border: InputBorder.none,
          isDense: true,
          contentPadding: const EdgeInsets.symmetric(vertical: Insets.md),
          hintText: hint,
          hintStyle: TextStyles.body.copyWith(color: palette.textTertiary),
        ),
      ),
    );
  }
}
