/// Motion, built the way Apple builds it.
///
/// Since iOS 17 Apple's default animation is a **spring**, specified by
/// `duration` and `bounce` rather than a Bézier curve. Three presets cover most
/// of what an app needs — `smooth` (critically damped, no overshoot), `snappy`
/// (a little bounce), and `bouncy` (more) — and each can be tuned by duration.
///
/// This reproduces that model so the app moves the way Apple's does, instead of
/// approximating it with `Curves.easeOut`, which is the wrong shape for anything
/// that responds to a person.
///
/// # Why a spring and not a curve
///
/// A spring is continuous *and* velocity-preserving. When a value is retargeted
/// mid-flight — a person opens a panel, then changes their mind — a spring
/// carries its current velocity into the new destination, so the motion stays
/// coherent. A fixed curve restarts from zero and visibly stutters.
///
/// Flutter's implicit animations take a [`Curve`], so the springs here are
/// expressed as curves that evaluate a real [`SpringSimulation`] rather than as a
/// ticker of our own. `Curve.transform` is allowed to return values above 1,
/// which is how the overshoot of an underdamped spring survives: `Curves.easeOutBack`,
/// which Apple's own controls resemble, does the same.
///
/// # Sources
///
/// - `motion.md`: "Add motion purposefully"; "Make motion optional"; "Aim for
///   brevity and precision in feedback animations"; "In apps, generally avoid
///   adding motion to UI interactions that occur frequently."
/// - `accessibility.md`: under Reduce Motion, "Tightening animation springs to
///   reduce bounce effects" and "Replacing transitions in x-, y-, and z-axes
///   with fades to avoid motion".
/// - WWDC23 "Animate with springs": the `duration`/`bounce` model, the three
///   presets, the note that bounces above ~0.4 "may feel too exaggerated for a UI
///   element", and that bounce 0 is the general-purpose choice.
library;

import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/physics.dart';

/// A spring in Apple's terms: how long it takes to settle, and how much it
/// overshoots.
@immutable
class AppleSpring {
  const AppleSpring({required this.duration, required this.bounce});

  /// How long the spring takes to settle, in seconds.
  final double duration;

  /// Overshoot, from -1.0 to 1.0. 0 is critically damped: the object arrives as
  /// fast as it can without crossing its target.
  final double bounce;

  /// `smooth`: critically damped, no overshoot.
  static const smooth = AppleSpring(duration: 0.5, bounce: 0.0);

  /// `snappy`: slightly underdamped. The general purpose preset, and Apple's
  /// default.
  static const snappy = AppleSpring(duration: 0.5, bounce: 0.15);

  /// `bouncy`: clearly underdamped, for a moment that should feel physical.
  static const bouncy = AppleSpring(duration: 0.5, bounce: 0.3);

  /// Tuned short, for control feedback. Apple asks for brevity on interactions
  /// that happen often, so press and hover states use this.
  static const instant = AppleSpring(duration: 0.22, bounce: 0.0);

  /// A control settling into place, with just enough life to read as physical.
  static const control = AppleSpring(duration: 0.32, bounce: 0.12);

  /// The damping ratio, derived from `bounce`.
  ///
  /// Apple maps bounce `b` onto a ratio of `1 - b` for `b >= 0`, which is why
  /// bounce 0 is critically damped and 0.3 is noticeably springy.
  double get dampingRatio => (1 - bounce).clamp(0.05, 2.0);

  /// A Flutter spring description with equivalent behaviour.
  ///
  /// Mass is fixed at 1 so the period is governed by stiffness alone, which is
  /// how Apple's two-parameter model behaves. For a damping ratio below 1 a
  /// spring settles to within 0.1% of its target in about `6.9 / (zeta * omega)`
  /// seconds, so the stiffness that settles in the requested time follows from
  /// inverting that.
  SpringDescription get description {
    final zeta = dampingRatio;
    final angular = 6.9 / (zeta * duration);
    return SpringDescription(mass: 1, stiffness: angular * angular, damping: 2 * zeta * angular);
  }

  /// This spring, with the bounce reduced by `amount`.
  ///
  /// Reduce Motion asks for exactly this: `accessibility.md` lists "tightening
  /// animation springs to reduce bounce effects" as one of the ways to honour
  /// the setting.
  AppleSpring tightened(double amount) =>
      AppleSpring(duration: duration, bounce: (bounce - amount).clamp(0.0, 1.0));

  /// This spring at a different speed, where above 1 is faster.
  AppleSpring speed(double factor) => AppleSpring(duration: duration / factor, bounce: bounce);

  /// This spring as a Flutter curve, ready for an implicit animation.
  Curve get curve => SpringCurve(this);

  @override
  String toString() => 'AppleSpring(${duration}s, bounce $bounce)';
}

/// A [`Curve`] that evaluates a spring.
///
/// The spring is sampled once and interpolated between samples. That is exact
/// enough to be indistinguishable — the simulation is smooth and the samples are
/// dense — and far cheaper than stepping the simulation on every frame of every
/// animating widget.
class SpringCurve extends Curve {
  /// Builds a curve for a spring.
  SpringCurve(this.spring);

  final AppleSpring spring;

  /// How many samples to precompute. Dense enough to look smooth, small enough
  /// that the cost is irrelevant.
  static const _samples = 120;

  /// The sampled spring, built on first use.
  late final List<double> _values = _build();

  List<double> _build() {
    final simulation = SpringSimulation(spring.description, 0, 1, 0);
    final settle = _settleTime(simulation);

    final values = List<double>.filled(_samples + 1, 1);
    for (var i = 0; i <= _samples; i++) {
      final at = settle * i / _samples;
      // Clamped below at 0 so a spring cannot push a value negative, and left
      // free above so the overshoot shows. Flutter's own overshoot curves do
      // the same.
      values[i] = math.max(0, simulation.x(at));
    }
    return values;
  }

  /// How long the simulation takes to come to rest, capped so a very stiff
  /// spring cannot spin here.
  static double _settleTime(SpringSimulation simulation) {
    const step = 1 / 240;
    const cap = 5.0;
    for (var t = 0.0; t < cap; t += step) {
      if (simulation.isDone(t)) return t;
    }
    return cap;
  }

  @override
  double transformInternal(double t) {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    final position = t * _samples;
    final lower = position.floor();
    final upper = math.min(lower + 1, _samples);
    final fraction = position - lower;
    return _values[lower] + (_values[upper] - _values[lower]) * fraction;
  }

  @override
  String toString() => 'SpringCurve($spring)';
}

/// The durations and springs the app uses, named for their job.
///
/// Apple's model is small on purpose: a few springs, each attached to a
/// behaviour rather than to a widget. Naming them by behaviour is what keeps a
/// new screen from inventing its own timing.
abstract final class Motion {
  /// Press and hover feedback. Apple asks for brevity on frequent interactions,
  /// so this is the shortest.
  static const Duration fast = Duration(milliseconds: 140);

  /// A panel or a colour arriving.
  static const Duration normal = Duration(milliseconds: 280);

  /// A page change: the largest movement in the app, so it gets the most time.
  static const Duration slow = Duration(milliseconds: 420);

  /// A stagger step, for a list arriving in sequence.
  static const Duration stagger = Duration(milliseconds: 40);

  /// Arrival. Ease-out is the right shape for a colour, an opacity, or a
  /// hairline — things where a spring's overshoot would be imperceptible but its
  /// settle time would not.
  static const Curve enter = Curves.easeOutCubic;

  /// Departure. Slightly quicker than arrival, so a thing leaving feels like it
  /// gave way rather than was pushed.
  static const Curve exit = Curves.easeInCubic;

  /// The curve Reduce Motion substitutes: a plain decelerating fade with no
  /// overshoot and no directional travel of its own.
  static const Curve reduced = Curves.easeOut;

  /// Direct manipulation and presses: quick, no overshoot.
  static final Curve interactive = AppleSpring.instant.curve;

  /// A control changing state — a selection, a toggle, a badge.
  static final Curve control = AppleSpring.control.curve;

  /// Content arriving in a view: sheets, rows, cards.
  static final Curve content = AppleSpring.snappy.curve;

  /// A page change.
  static final Curve page = AppleSpring.smooth.curve;

  /// A moment that should feel physical.
  static final Curve playful = AppleSpring.bouncy.curve;
}

/// Motion, as the platform wants it.
///
/// `motion.md`: "Make motion optional". `accessibility.md` requires Reduce
/// Motion to reduce automatic and repetitive animation, and names two of the
/// techniques used here: tightening springs, and replacing movement with a fade.
///
/// Read through the context rather than captured, so a settings change is picked
/// up without a restart.
extension MotionContext on BuildContext {
  /// Whether the platform asks for reduced motion.
  bool get reduceMotion => MediaQuery.maybeDisableAnimationsOf(this) ?? false;

  /// The curve to use, or a decelerating fade when motion is reduced.
  ///
  /// The substitute is still a curve, not `Curves.linear`: the setting asks for
  /// less movement, not for mechanical movement.
  Curve motionCurve(Curve preferred) => reduceMotion ? Motion.reduced : preferred;

  /// A duration, shortened when motion is reduced. The animation still happens —
  /// it simply gets out of the way sooner.
  Duration motionDuration(Duration preferred) => reduceMotion
      ? Duration(milliseconds: (preferred.inMilliseconds * 0.6).round())
      : preferred;
}
