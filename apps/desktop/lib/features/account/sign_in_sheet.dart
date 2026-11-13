import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/engine.dart';
// The event DTO the bridge emits. Named explicitly rather than wildcard-imported,
// so a change to the bridge surface shows up here as a compile error.
import '../../rust/api.dart' show SignInEventDto;
import '../../theme/theme.dart';

/// Native sign-in to Apple, through the `FairPlay` bridge.
///
/// This is the **native** path, not a web view: Apple's own account libraries do
/// the authentication, which is why it works on Linux at all. The web route is
/// closed because WebKitGTK ships without Encrypted Media, so Apple's sign-in
/// page cannot finish. See `docs/PLATFORM-LIMITS.md`.
///
/// The flow is three steps, and the second only appears if Apple asks for it:
///
/// 1. Email and password.
/// 2. A verification code, when a trusted device is challenged.
/// 3. Done, or Apple's own message explaining what went wrong.
class SignInSheet extends ConsumerStatefulWidget {
  const SignInSheet({super.key});

  /// Opens the sheet. The result is `true` when sign-in succeeded, so the caller
  /// can refresh what it shows.
  static Future<bool> show(BuildContext context) async {
    final result = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      barrierColor: Colors.black.withValues(alpha: 0.45),
      builder: (context) => const SignInSheet(),
    );
    return result ?? false;
  }

  @override
  ConsumerState<SignInSheet> createState() => _SignInSheetState();
}

/// Which step the sheet is on.
enum _Step { credentials, code, working }

class _SignInSheetState extends ConsumerState<SignInSheet> {
  final _email = TextEditingController();
  final _password = TextEditingController();
  final _code = TextEditingController();

  _Step _step = _Step.credentials;
  String? _error;
  StreamSubscription<SignInEventDto>? _events;
  bool _obscure = true;

  @override
  void dispose() {
    _events?.cancel();
    _email.dispose();
    _password.dispose();
    _code.dispose();
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
              _Header(step: _step),
              const SizedBox(height: Insets.md),
              _Explanation(step: _step),
              const SizedBox(height: Insets.lg),
              // The fields are dropped once the credentials are submitted. Leaving
              // them on screen during the wait keeps a typed password visible for
              // no reason, and there is nothing to edit while the call is in
              // flight.
              if (_step == _Step.code)
                _CodeField(controller: _code)
              else if (_step == _Step.credentials) ...[
                _Field(
                  controller: _email,
                  hint: 'Apple ID',
                  enabled: true,
                ),
                const SizedBox(height: Insets.sm),
                _Field(
                  controller: _password,
                  hint: 'Password',
                  enabled: true,
                  obscure: _obscure,
                  onToggleObscure: () => setState(() => _obscure = !_obscure),
                ),
              ],
              if (_error != null) ...[
                const SizedBox(height: Insets.md),
                _ErrorLine(message: _error!),
              ],
              const SizedBox(height: Insets.lg),
              _Actions(
                step: _step,
                onCancel: _cancel,
                onContinue: _continue,
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Moves to the next step, or performs it when there is nothing to collect.
  Future<void> _continue() async {
    switch (_step) {
      case _Step.credentials:
        final email = _email.text.trim();
        final password = _password.text;
        if (email.isEmpty || password.isEmpty) {
          setState(() => _error = 'Enter your Apple ID and password to continue.');
          return;
        }
        await _start(email, password);
      case _Step.code:
        final code = _code.text.trim();
        if (code.isEmpty) {
          setState(() => _error = 'Enter the code Apple sent to your devices.');
          return;
        }
        await _submitCode(code);
      case _Step.working:
        break;
    }
  }

  Future<void> _start(String email, String password) async {
    setState(() {
      _step = _Step.working;
      _error = null;
    });

    final engine = ref.read(engineProvider).requireValue;

    // Subscribe before starting, so an event emitted immediately is not missed.
    _events = engine.watchSignIn().listen(_onEvent);

    try {
      await engine.beginSignIn(email: email, password: password);
    } on Exception catch (error) {
      if (!mounted) return;
      setState(() {
        _step = _Step.credentials;
        _error = _describe(error);
      });
    }
  }

  Future<void> _submitCode(String code) async {
    final engine = ref.read(engineProvider).requireValue;
    setState(() => _error = null);
    try {
      await engine.submitSignInCode(code: code);
    } on Exception catch (error) {
      if (!mounted) return;
      setState(() => _error = _describe(error));
    }
  }

  void _onEvent(SignInEventDto event) {
    if (!mounted) return;
    switch (event.kind) {
      case 'code_required':
        // Only now does the code field appear, so the user is never asked for a
        // code the bridge is not yet waiting for.
        setState(() {
          _step = _Step.code;
          _error = null;
        });
      case 'preparing':
        setState(() => _step = _Step.working);
      case 'failed':
        // Apple's own wording, where there was any: it is far more useful than a
        // generic failure, and the bridge already extracted it.
        setState(() {
          _step = _Step.credentials;
          _error = event.message ?? 'Sign-in failed.';
        });
      case 'succeeded':
        Navigator.of(context).pop(true);
      case 'started':
      default:
        break;
    }
  }

  void _cancel() {
    // Abandon rather than just close: the worker is blocked waiting for an
    // answer, and dropping the sheet without telling it would leave it there.
    ref.read(engineProvider).requireValue.cancelSignIn();
    Navigator.of(context).pop(false);
  }

  /// Turns a bridge error into something worth reading.
  ///
  /// The bridge sends a `kind` for each failure, and the remedies differ, so the
  /// kind decides the wording rather than the raw message.
  String _describe(Exception error) {
    final text = error.toString();
    if (text.contains('not installed')) {
      return 'The lossless component is not installed. Install it from Settings '
          'and try again.';
    }
    if (text.contains('already in progress')) {
      return 'A sign-in is already in progress.';
    }
    return text;
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.step});

  final _Step step;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final title = switch (step) {
      _Step.credentials => 'Sign in to Apple Music',
      _Step.code => 'Enter the verification code',
      _Step.working => 'Signing in\u2026',
    };

    return Row(
      children: [
        Icon(Icons.apple, size: 20, color: palette.textPrimary),
        const SizedBox(width: Insets.sm),
        Expanded(
          child: Text(title, style: TextStyles.title3.copyWith(color: palette.textPrimary)),
        ),
      ],
    );
  }
}

class _Explanation extends StatelessWidget {
  const _Explanation({required this.step});

  final _Step step;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final text = switch (step) {
      _Step.credentials =>
        'Sosumi plays music from your Apple Music subscription. Your credentials '
            'go directly to Apple and are never stored by Sosumi.',
      _Step.code =>
        'Apple sent a code to your trusted devices. Enter it here to finish '
            'signing in.',
      _Step.working =>
        'Talking to Apple. This can take a few seconds.',
    };

    return Text(text, style: TextStyles.callout.copyWith(color: palette.textSecondary));
  }
}

class _Field extends StatelessWidget {
  const _Field({
    required this.controller,
    required this.hint,
    required this.enabled,
    this.obscure = false,
    this.onToggleObscure,
  });

  final TextEditingController controller;
  final String hint;
  final bool enabled;
  final bool obscure;
  final VoidCallback? onToggleObscure;

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
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: controller,
              enabled: enabled,
              obscureText: obscure,
              autocorrect: false,
              // The Apple ID is an email address, so the keyboard and the
              // autofill hints should say so rather than offering a username.
              keyboardType:
                  hint == 'Apple ID' ? TextInputType.emailAddress : TextInputType.text,
              autofillHints: hint == 'Apple ID'
                  ? const [AutofillHints.username, AutofillHints.email]
                  : null,
              style: TextStyles.body.copyWith(color: palette.textPrimary),
              cursorColor: palette.accent,
              onSubmitted: (_) => onToggleObscure == null ? null : null,
              decoration: InputDecoration(
                border: InputBorder.none,
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(vertical: Insets.md),
                hintText: hint,
                hintStyle: TextStyles.body.copyWith(color: palette.textTertiary),
              ),
            ),
          ),
          if (onToggleObscure != null)
            IconButton(
              onPressed: onToggleObscure,
              icon: Icon(
                obscure ? Icons.visibility_outlined : Icons.visibility_off_outlined,
                size: 15,
                color: palette.textTertiary,
              ),
              tooltip: obscure ? 'Show password' : 'Hide password',
              splashRadius: 16,
            ),
        ],
      ),
    );
  }
}

class _CodeField extends StatelessWidget {
  const _CodeField({required this.controller});

  final TextEditingController controller;

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
        autofocus: true,
        keyboardType: TextInputType.number,
        // A verification code is one-time, so it is never autofilled.
        autofillHints: null,
        maxLength: 6,
        style: TextStyles.title3.copyWith(
          color: palette.textPrimary,
          letterSpacing: 6,
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
        cursorColor: palette.accent,
        textAlign: TextAlign.center,
        decoration: InputDecoration(
          border: InputBorder.none,
          isDense: true,
          counterText: '',
          contentPadding: const EdgeInsets.symmetric(vertical: Insets.md),
          hintText: '000000',
          hintStyle: TextStyles.title3.copyWith(
            color: palette.textTertiary,
            letterSpacing: 6,
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
      ),
    );
  }
}

class _ErrorLine extends StatelessWidget {
  const _ErrorLine({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(Icons.warning_amber_rounded, size: 13, color: palette.accent),
        const SizedBox(width: Insets.xs),
        Expanded(
          child: Text(message, style: TextStyles.caption1.copyWith(color: palette.accent)),
        ),
      ],
    );
  }
}

class _Actions extends StatelessWidget {
  const _Actions({
    required this.step,
    required this.onCancel,
    required this.onContinue,
  });

  final _Step step;
  final VoidCallback onCancel;
  final VoidCallback onContinue;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final busy = step == _Step.working;

    return Row(
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        TextButton(
          onPressed: onCancel,
          child: Text(
            busy ? 'Cancel' : 'Not Now',
            style: TextStyles.body.copyWith(color: palette.textSecondary),
          ),
        ),
        const SizedBox(width: Insets.sm),
        FilledButton(
          onPressed: busy ? null : onContinue,
          style: FilledButton.styleFrom(
            backgroundColor: palette.accent,
            foregroundColor: Colors.white,
            disabledBackgroundColor: palette.fill,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(Radii.small),
            ),
            padding: const EdgeInsets.symmetric(
              horizontal: Insets.lg,
              vertical: Insets.md,
            ),
          ),
          child: busy
              ? const SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                )
              : Text(step == _Step.code ? 'Verify' : 'Continue'),
        ),
      ],
    );
  }
}
