import 'dart:math' as math;

import 'package:flutter/material.dart';

/// Procedural cover art used by the scaffold.
///
/// The real app will render Apple Music artwork. Until the catalog is wired,
/// these deterministic covers give the layout real visual weight without
/// shipping borrowed artwork.
class AlbumArt extends StatelessWidget {
  const AlbumArt({
    super.key,
    required this.seed,
    this.size,
    this.radius = 12,
    this.overlay,
  });

  final int seed;
  final double? size;
  final double radius;
  final Widget? overlay;

  @override
  Widget build(BuildContext context) {
    final child = ClipRRect(
      borderRadius: BorderRadius.circular(radius),
      child: CustomPaint(
        painter: _CoverPainter(seed: seed),
        child: overlay == null
            ? const SizedBox.expand()
            : Align(alignment: Alignment.bottomLeft, child: overlay),
      ),
    );

    if (size == null) return child;
    return SizedBox.square(dimension: size, child: child);
  }
}

class _CoverPainter extends CustomPainter {
  _CoverPainter({required this.seed});

  final int seed;

  static const _palettes = <List<Color>>[
    [Color(0xFFFF2D55), Color(0xFF7A1028), Color(0xFF12040A)],
    [Color(0xFF4F8CFF), Color(0xFF122B63), Color(0xFF05080F)],
    [Color(0xFF37D6B0), Color(0xFF0D4A43), Color(0xFF04100E)],
    [Color(0xFFFFB340), Color(0xFF7A3C08), Color(0xFF120803)],
    [Color(0xFFB36BFF), Color(0xFF3A1766), Color(0xFF0A0412)],
    [Color(0xFFFF6B9A), Color(0xFF6B173D), Color(0xFF12040A)],
    [Color(0xFF9BD45B), Color(0xFF345B14), Color(0xFF060F03)],
    [Color(0xFF6EE7FF), Color(0xFF0A4A63), Color(0xFF020D12)],
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final random = math.Random(seed * 7919 + 17);
    final palette = _palettes[seed % _palettes.length];
    final rect = Offset.zero & size;

    canvas.drawRect(
      rect,
      Paint()
        ..shader = LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [palette[0], palette[1], palette[2]],
          stops: const [0, 0.58, 1],
        ).createShader(rect),
    );

    // Soft light field.
    final glowCenter = Offset(
      size.width * (0.18 + random.nextDouble() * 0.64),
      size.height * (0.12 + random.nextDouble() * 0.44),
    );
    final glowPaint = Paint()
      ..shader =
          RadialGradient(
            colors: [
              Colors.white.withValues(alpha: 0.42),
              palette[0].withValues(alpha: 0.16),
              Colors.transparent,
            ],
            stops: const [0, 0.42, 1],
          ).createShader(
            Rect.fromCircle(
              center: glowCenter,
              radius: size.shortestSide * 0.75,
            ),
          );
    canvas.drawCircle(glowCenter, size.shortestSide * 0.75, glowPaint);

    // Geometric mark: rings, a diagonal band, and a few specks.
    final ringPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = size.shortestSide * 0.018
      ..color = Colors.white.withValues(alpha: 0.26);
    final ringCenter = Offset(
      size.width * (0.25 + random.nextDouble() * 0.5),
      size.height * (0.35 + random.nextDouble() * 0.35),
    );
    final ringRadius = size.shortestSide * (0.18 + random.nextDouble() * 0.22);
    canvas.drawCircle(ringCenter, ringRadius, ringPaint);
    canvas.drawCircle(
      ringCenter,
      ringRadius * 0.62,
      ringPaint..color = Colors.white.withValues(alpha: 0.13),
    );

    final band = Path()
      ..moveTo(-size.width * 0.1, size.height * 0.78)
      ..lineTo(
        size.width * 1.1,
        size.height * (0.18 + random.nextDouble() * 0.2),
      )
      ..lineTo(size.width * 1.1, size.height * 0.30)
      ..lineTo(-size.width * 0.1, size.height * 0.90)
      ..close();
    canvas.drawPath(
      band,
      Paint()..color = Colors.black.withValues(alpha: 0.16),
    );

    for (var i = 0; i < 18; i++) {
      final dot = Offset(
        size.width * random.nextDouble(),
        size.height * random.nextDouble(),
      );
      canvas.drawCircle(
        dot,
        size.shortestSide * (0.002 + random.nextDouble() * 0.006),
        Paint()..color = Colors.white.withValues(alpha: 0.12),
      );
    }

    // Subtle vignette.
    canvas.drawRect(
      rect,
      Paint()
        ..shader = RadialGradient(
          colors: [Colors.transparent, Colors.black.withValues(alpha: 0.38)],
          stops: const [0.58, 1],
        ).createShader(rect),
    );
  }

  @override
  bool shouldRepaint(covariant _CoverPainter oldDelegate) =>
      oldDelegate.seed != seed;
}
