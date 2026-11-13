import 'package:flutter/material.dart';

import 'motion.dart';

/// Reveals its child on first build, optionally after a delay.
///
/// Apple staggers content that arrives together — a list of rows appears in
/// sequence rather than all at once — which reads as the interface assembling
/// itself rather than blinking into existence. `motion.md` allows this as long
/// as it is purposeful: "Add motion purposefully, supporting the experience
/// without overshadowing it."
///
/// Two rules keep it from becoming a nuisance:
///
/// - It runs **once**, on first build. Re-entering a screen does not replay it,
///   because `motion.md` warns against animating "interactions that occur
///   frequently".
/// - It is **bounded**: a long list is not staggered forever. Beyond
///   [maxStaggered] the delay stops growing, so the last row of a 200-item list
///   arrives with the tenth rather than minutes later.
class Reveal extends StatefulWidget {
  const Reveal({
    super.key,
    required this.child,
    this.index = 0,
    this.rise = 0.03,
  });

  final Widget child;

  /// Position in the sequence, which sets the delay.
  final int index;

  /// How far the child rises into place, as a fraction of its own height.
  final double rise;

  /// The most items that are staggered. Past this the delay is capped.
  static const maxStaggered = 8;

  @override
  State<Reveal> createState() => _RevealState();
}

class _RevealState extends State<Reveal> {
  /// Whether the entry has played. Kept so a rebuild does not replay it.
  bool _shown = false;

  @override
  void initState() {
    super.initState();
    // A post-frame start, so the first frame paints at the resting (hidden)
    // position and the animation has somewhere to come from.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) setState(() => _shown = true);
    });
  }

  @override
  Widget build(BuildContext context) {
    // Reduce Motion asks for no travel at all, so the delay is dropped and the
    // child simply fades. `accessibility.md`: "Replacing transitions in x-, y-,
    // and z-axes with fades to avoid motion".
    final reduced = context.reduceMotion;
    final delay = reduced
        ? Duration.zero
        : Motion.stagger * widget.index.clamp(0, Reveal.maxStaggered);

    return AnimatedSlide(
      offset: _shown ? Offset.zero : Offset(0, widget.rise),
      duration: context.motionDuration(Motion.normal) + delay,
      curve: context.motionCurve(Motion.content),
      child: AnimatedOpacity(
        opacity: _shown ? 1 : 0,
        duration: context.motionDuration(Motion.normal) + delay,
        curve: Motion.enter,
        child: widget.child,
      ),
    );
  }
}
