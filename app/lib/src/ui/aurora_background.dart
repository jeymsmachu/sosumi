import 'dart:math' as math;
import 'dart:ui';

import 'package:flutter/material.dart';

/// A slow-moving gradient field behind the app chrome.
///
/// It is intentionally cheap: three blurred circles, no shaders. Motion can be
/// disabled from Settings without changing the layout.
class AuroraBackground extends StatefulWidget {
  const AuroraBackground({
    super.key,
    required this.accent,
    required this.animate,
    required this.child,
  });

  final Color accent;
  final bool animate;
  final Widget child;

  @override
  State<AuroraBackground> createState() => _AuroraBackgroundState();
}

class _AuroraBackgroundState extends State<AuroraBackground>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 24),
    );
    _syncAnimation();
  }

  @override
  void didUpdateWidget(covariant AuroraBackground oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.animate != widget.animate) {
      _syncAnimation();
    }
  }

  void _syncAnimation() {
    if (widget.animate) {
      _controller.repeat(reverse: true);
    } else {
      _controller
        ..stop()
        ..value = 0.34;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: const Color(0xFF07070A),
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, _) {
          final phase = _controller.value * math.pi * 2;
          final drift = math.sin(phase) * 34;
          final counterDrift = math.cos(phase) * 26;

          return Stack(
            fit: StackFit.expand,
            children: [
              _Orb(
                top: -170 + drift,
                left: -130,
                size: 560,
                color: widget.accent.withValues(alpha: 0.30),
              ),
              _Orb(
                top: 60 - counterDrift,
                right: -190,
                size: 600,
                color: const Color(0xFF2D6BFF).withValues(alpha: 0.19),
              ),
              _Orb(
                bottom: -250 + drift,
                left: 100 + counterDrift,
                size: 640,
                color: const Color(0xFFFF6B9A).withValues(alpha: 0.14),
              ),
              Positioned.fill(child: widget.child),
            ],
          );
        },
      ),
    );
  }
}

class _Orb extends StatelessWidget {
  const _Orb({
    required this.size,
    required this.color,
    this.top,
    this.left,
    this.right,
    this.bottom,
  });

  final double size;
  final Color color;
  final double? top;
  final double? left;
  final double? right;
  final double? bottom;

  @override
  Widget build(BuildContext context) {
    return Positioned(
      top: top,
      left: left,
      right: right,
      bottom: bottom,
      width: size,
      height: size,
      child: IgnorePointer(
        child: ImageFiltered(
          imageFilter: ImageFilter.blur(sigmaX: 90, sigmaY: 90),
          child: DecoratedBox(
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              gradient: RadialGradient(
                colors: [color, color.withValues(alpha: 0)],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
