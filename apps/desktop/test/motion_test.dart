import 'package:flutter_test/flutter_test.dart';
import 'package:sosumi/theme/motion.dart';

/// The spring model, which the whole app's motion now rests on.
///
/// These assert the *shape* of the motion rather than exact samples, because the
/// exact numbers come from Flutter's simulation and would only be re-stating the
/// implementation. What matters is that the presets behave the way Apple
/// describes them: smooth does not overshoot, snappy overshoots a little, bouncy
/// overshoots more, and all of them start at 0 and end at 1.
void main() {
  group('AppleSpring', () {
    test('every preset starts at rest and arrives exactly', () {
      for (final spring in [
        AppleSpring.smooth,
        AppleSpring.snappy,
        AppleSpring.bouncy,
        AppleSpring.instant,
        AppleSpring.control,
      ]) {
        final curve = spring.curve;
        expect(curve.transform(0), 0, reason: '$spring must start at 0');
        expect(curve.transform(1), 1, reason: '$spring must arrive at 1');
      }
    });

    test('smooth does not overshoot', () {
      // Critically damped: Apple's "clean, settled arrival". A single sample
      // above 1 would be the bounce the preset exists to avoid.
      final curve = AppleSpring.smooth.curve;
      for (var t = 0.0; t <= 1.0; t += 0.005) {
        expect(
          curve.transform(t),
          lessThanOrEqualTo(1.0 + 1e-9),
          reason: 'smooth overshot at t=$t',
        );
      }
    });

    test('snappy and bouncy overshoot, and bouncy overshoots more', () {
      double peak(AppleSpring spring) {
        var highest = 0.0;
        for (var t = 0.0; t <= 1.0; t += 0.002) {
          highest = highest > spring.curve.transform(t) ? highest : spring.curve.transform(t);
        }
        return highest;
      }

      final snappy = peak(AppleSpring.snappy);
      final bouncy = peak(AppleSpring.bouncy);

      expect(snappy, greaterThan(1.0), reason: 'snappy is meant to be slightly underdamped');
      expect(bouncy, greaterThan(snappy), reason: 'bouncy is the more energetic preset');
      // Apple's own ceiling: a bounce above roughly 0.4 reads as exaggerated for
      // a UI element, so the presets stay under it.
      expect(bouncy, lessThan(1.4));
    });

    test('motion is monotonic in time for a critically damped spring', () {
      // A settled arrival should not go backwards, which is what distinguishes
      // it from a bouncy one.
      final curve = AppleSpring.smooth.curve;
      var previous = 0.0;
      for (var t = 0.0; t <= 1.0; t += 0.005) {
        final value = curve.transform(t);
        expect(value, greaterThanOrEqualTo(previous - 1e-9), reason: 'regressed at t=$t');
        previous = value;
      }
    });

    test('tightening removes bounce without touching the duration', () {
      // This is the Reduce Motion mechanism, so it must actually flatten.
      final tightened = AppleSpring.bouncy.tightened(AppleSpring.bouncy.bounce);
      expect(tightened.bounce, 0);
      expect(tightened.duration, AppleSpring.bouncy.duration);
    });

    test('speed scales the settle time, not the bounce', () {
      final faster = AppleSpring.snappy.speed(2);
      expect(faster.duration, closeTo(AppleSpring.snappy.duration / 2, 1e-9));
      expect(faster.bounce, AppleSpring.snappy.bounce);
    });

    test('bounce 0 is critically damped', () {
      // The mapping Apple documents: bounce b becomes a damping ratio of 1 - b.
      expect(AppleSpring.smooth.dampingRatio, closeTo(1.0, 1e-9));
      expect(AppleSpring.bouncy.dampingRatio, closeTo(0.7, 1e-9));
    });

    test('an extreme bounce still yields a usable spring description', () {
      // Guards the division: a bounce of 1 would give a damping ratio of 0 and
      // an infinite angular frequency without the clamp.
      const extreme = AppleSpring(duration: 0.5, bounce: 1.0);
      final described = extreme.description;
      expect(described.stiffness.isFinite, isTrue);
      expect(described.damping.isFinite, isTrue);
    });
  });
}
