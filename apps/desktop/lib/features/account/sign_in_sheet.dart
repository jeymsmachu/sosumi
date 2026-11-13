import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/engine.dart';
import '../../core/install.dart';
// The event DTO the bridge emits. Named explicitly rather than wildcard-imported,
// so a change to the bridge surface shows up here as a compile error.
import '../../rust/api.dart' show SignInEventDto;
import '../../theme/theme.dart';
import 'install_progress.dart';

/// Native sign-in to Apple, through the `FairPlay` bridge.
///
/// This is the **native** path, not a web view: Apple's own account libraries do
/// the authentication, which is why it works on Linux at all. The web route is
/// closed because WebKitGTK ships without Encrypted Media, so Apple's sign-in
/// page cannot finish. See `docs/PLATFORM-LIMITS.md`.
///
/// # Why the prerequisite is checked before the form
///
/// Sign-in is performed by Apple's libraries, which are not bundled. Asking for
/// an Apple ID and password and *then* saying a module is missing reads as a
/// wrong password and sends the person to Settings to fix something they were
/// never told about. `entering-data.md` is direct about this: validate before
/// collecting, and explain in place.
///
/// So the sheet opens on whichever step is true. If the component is missing it
/// leads with that, offers the install, and shows real progress — then continues
/// into the form on its own.
class SignInSheet extends ConsumerStatefulWidget {
  const SignInSheet({super.key});

  /// Opens the sheet. The result is `true` when sign-in succeeded, so the caller
  /// can adopt the session.
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
enum _Step {
  /// The required component is missing but installable. Shown first, and only
  /// when it is.
  prepare,

  /// The machine cannot run the component at all. There is no action to offer, so
  /// the sheet says so rather than showing a button that cannot work.
  unsupported,

  /// Apple ID and password.
  credentials,

  /// The code Apple sent to a trusted device.
  code,

  /// Waiting on Apple.
  working,
}

class _SignInSheetState extends ConsumerState<SignInSheet> {
  final _email = TextEditingController();
  final _password = TextEditingController();
  final _code = TextEditingController();

  _Step _step = _Step.credentials;
  String? _error;
  StreamSubscription<SignInEventDto>? _events;
  bool _obscure = true;

  @override
  void initState() {
    super.initState();
    // Checked after the first frame rather than in `initState`, because the
    // check crosses the bridge and `build` must not await it. The sheet opens on
    // the credential fields and moves to the prepare step only if the check says
    // the component is missing, which keeps the common case free of a flash.
    WidgetsBinding.instance.addPostFrameCallback((_) => _checkReady());
  }

  @override
  void dispose() {
    _events?.cancel();
    _email.dispose();
    _password.dispose();
    _code.dispose();
    super.dispose();
  }

  /// Asks the bridge how ready sign-in is, and opens on the right step.
  Future<void> _checkReady() async {
    if (!mounted) return;
    final engine = ref.read(engineProvider).requireValue;
    final readiness = await engine.signInReadiness();
    if (!mounted) return;
    setState(() {
      _step = switch (readiness) {
        'ready' => _Step.credentials,
        'unsupported' => _Step.unsupported,
        // `installable` and `damaged` both lead to the install step: a damaged
        // payload needs the same fetch as a missing one, and the fetch skips the
        // files that already verify.
        _ => _Step.prepare,
      };
    });
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    // The install state drives the prepare step's progress row, so the sheet
    // watches it rather than owning a copy.
    final install = ref.watch(installProvider);

    // The install finishing is what moves the sheet on, so no button is needed
    // to continue: the state the person was waiting for has arrived.
    ref.listen(installProvider, (previous, next) {
      if (previous?.isRunning == true && next.phase == InstallPhase.done) {
        _checkReady();
      }
    });

    return Dialog(
      backgroundColor: Colors.transparent,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 400),
        child: GlassSurface(
          radius: Radii.panel,
          blur: 44,
          fill: palette.elevatedGlass,
          padding: const EdgeInsets.all(Insets.lg),
          child: AnimatedSize(
            duration: context.motionDuration(Motion.normal),
            curve: context.motionCurve(Motion.content),
            alignment: Alignment.topCenter,
            child: AnimatedSwitcher(
              duration: context.motionDuration(Motion.normal),
              switchInCurve: context.motionCurve(Motion.content),
              switchOutCurve: Motion.exit,
              // The default fade would cross-fade two differently sized panels
              // into a smear; a fade plus a small rise reads as one panel
              // changing its contents.
              transitionBuilder: (child, animation) => FadeTransition(
                opacity: animation,
                child: SlideTransition(
                  position: Tween(
                    begin: const Offset(0, 0.04),
                    end: Offset.zero,
                  ).animate(animation),
                  child: child,
                ),
              ),
              child: Column(
                key: ValueKey(_step),
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _Header(step: _step),
                  const SizedBox(height: Insets.md),
                  _Explanation(step: _step),
                  const SizedBox(height: Insets.lg),
                  ..._body(install),
                  if (_error != null) ...[
                    const SizedBox(height: Insets.md),
                    _ErrorLine(message: _error!),
                  ],
                  const SizedBox(height: Insets.lg),
                  _Actions(
                    step: _step,
                    busy: _step == _Step.working || install.isRunning,
                    onCancel: _cancel,
                    onContinue: _continue,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  /// The fields for the current step, if it has any.
  List<Widget> _body(InstallState install) {
    switch (_step) {
      case _Step.unsupported:
        // No action is offered, so there is nothing here: a button would be a
        // promise the machine cannot keep.
        return const [];
      case _Step.prepare:
        return [
          InstallProgress(state: install),
          if (install.phase == InstallPhase.failed) ...[
            const SizedBox(height: Insets.md),
            _ErrorLine(message: install.error ?? 'The download failed.'),
          ],
        ];
      case _Step.code:
        return [_CodeField(controller: _code)];
      case _Step.working:
        // Nothing to edit while the call is in flight, and leaving a typed
        // password on screen during the wait serves no purpose.
        return const [];
      case _Step.credentials:
        return [
          _Field(controller: _email, hint: 'Apple ID', enabled: true, onSubmitted: _continue),
          const SizedBox(height: Insets.sm),
          _Field(
            controller: _password,
            hint: 'Password',
            enabled: true,
            obscure: _obscure,
            onSubmitted: _continue,
            onToggleObscure: () => setState(() => _obscure = !_obscure),
          ),
        ];
    }
  }

  /// Moves to the next step, or performs it when there is nothing to collect.
  Future<void> _continue() async {
    switch (_step) {
      case _Step.unsupported:
        // Unreachable: the step offers no action. Handled rather than left out so
        // a new step cannot silently fall through to starting an install.
        break;
      case _Step.prepare:
        await _startInstall();
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

  /// Begins the install the prepare step is offering.
  Future<void> _startInstall() async {
    setState(() => _error = null);
    await ref.read(installProvider.notifier).start();
  }

  Future<void> _start(String email, String password) async {
    setState(() {
      _step = _Step.working;
      _error = null;
    });

    final engine = ref.read(engineProvider).requireValue;

    // Cancel any previous subscription first. A retry reaches here again, and
    // leaving the old listener attached would handle every event twice — two
    // pops on success, which closes a screen the user did not ask to close.
    await _events?.cancel();

    // Subscribed before starting, so an event emitted immediately is not missed.
    // This is safe in that order because the channel belongs to the engine rather
    // than to the attempt.
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
  /// kind decides the wording rather than the raw message. The prerequisite case
  /// no longer reaches here — it is caught before the form — but the mapping
  /// stays for the rare race where the payload is removed mid-attempt.
  String _describe(Exception error) {
    final text = error.toString();
    if (text.contains('not installed') || text.contains('runtime is not installed')) {
      return 'Apple\'s sign-in module is missing. Sosumi offers to install it here.';
    }
    if (text.contains('already in progress')) {
      return 'A sign-in is already in progress.';
    }
    return text;
  }}

class _Header extends StatelessWidget {
  const _Header({required this.step});

  final _Step step;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final title = switch (step) {
      _Step.unsupported => 'Sign-in is unavailable',
      _Step.prepare => 'One component needed',
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
      // Says what is wrong and that there is nothing to be done here, rather
      // than offering a fix that cannot work. `writing.md`: "be clear about what
      // someone can do to fix it", and where there is nothing, say that plainly.
      _Step.unsupported =>
        'Apple\'s sign-in libraries are x86-64 only, and this machine is not. '
            'Sosumi cannot sign in here, so lossless playback is unavailable.',
      // States the benefit before the cost, and says why the download is needed
      // rather than presenting it as a rule.
      _Step.prepare =>
        'Apple performs the sign-in itself, through its own libraries. Sosumi '
            'does not ship those, so it fetches them to your machine once — about '
            '60 MB — and verifies every file. This is the same step that enables '
            'lossless playback.',
      _Step.credentials =>
        'Sosumi plays music from your Apple Music subscription. Your credentials '
            'go directly to Apple and are never stored by Sosumi.',
      _Step.code =>
        'Apple sent a code to your trusted devices. Enter it here to finish '
            'signing in.',
      _Step.working => 'Talking to Apple. This can take a few seconds.',
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
    this.onSubmitted,
  });

  final TextEditingController controller;
  final String hint;
  final bool enabled;
  final bool obscure;
  final VoidCallback? onToggleObscure;
  final VoidCallback? onSubmitted;

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
              // Enter submits, which is what a person expects in a short form
              // and what `keyboards.md` asks for on a desktop.
              onSubmitted: onSubmitted == null ? null : (_) => onSubmitted!(),
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

class _Actions extends ConsumerWidget {
  const _Actions({
    required this.step,
    required this.busy,
    required this.onCancel,
    required this.onContinue,
  });

  final _Step step;
  final bool busy;
  final VoidCallback onCancel;
  final VoidCallback onContinue;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    final install = ref.watch(installProvider);
    final installing = install.isRunning;

    // The primary label names what happens, per `writing.md`: "Every label says
    // what happens". "Continue" gives way to "Install" and then to a live file
    // count, so the button never asks for something the screen has not explained.
    final (label, onPressed) = switch (step) {
      // Nothing to do on a machine that cannot run it, so no primary action.
      _Step.unsupported => (null, null),
      _Step.prepare when install.phase == InstallPhase.done => ('Continue', onContinue),
      _Step.prepare when install.phase == InstallPhase.failed => ('Try Again', onContinue),
      _Step.prepare when installing => ('Installing\u2026', null),
      _Step.prepare => ('Install', onContinue),
      _Step.code => ('Verify', onContinue),
      _Step.working => ('Continue', null),
      _Step.credentials => ('Sign In', onContinue),
    };

    return Row(
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        // Cancel while an install is running actually stops it, so the label is
        // accurate rather than a dismissal that leaves a download going.
        TextButton(
          onPressed: installing ? () => ref.read(installProvider.notifier).cancel() : onCancel,
          child: Text(
            installing ? 'Stop' : (step == _Step.working ? 'Cancel' : 'Not Now'),
            style: TextStyles.body.copyWith(color: palette.textSecondary),
          ),
        ),
        if (label != null) ...[
          const SizedBox(width: Insets.sm),
          FilledButton(
            onPressed: busy ? null : onPressed,
            style: FilledButton.styleFrom(
              backgroundColor: palette.accent,
              foregroundColor: Colors.white,
              disabledBackgroundColor: palette.fill,
              disabledForegroundColor: palette.textTertiary,
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
                : Text(label),
          ),
        ],
      ],
    );
  }
}
