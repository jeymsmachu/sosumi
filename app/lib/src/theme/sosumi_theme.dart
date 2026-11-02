import 'package:flutter/material.dart';

/// Builds the dark, Apple-inspired theme used across every platform.
ThemeData buildSosumiTheme({required Color accent}) {
  final scheme =
      ColorScheme.fromSeed(
        seedColor: accent,
        brightness: Brightness.dark,
      ).copyWith(
        surface: const Color(0xFF0C0C11),
        surfaceContainerHighest: const Color(0xFF17171F),
      );

  final base = ThemeData(
    brightness: Brightness.dark,
    useMaterial3: true,
  ).textTheme;

  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: Colors.transparent,
    canvasColor: Colors.transparent,
    textTheme: base.copyWith(
      displaySmall: base.displaySmall?.copyWith(
        fontSize: 42,
        fontWeight: FontWeight.w700,
        letterSpacing: -1.6,
        height: 1.04,
      ),
      headlineMedium: base.headlineMedium?.copyWith(
        fontSize: 28,
        fontWeight: FontWeight.w700,
        letterSpacing: -0.8,
      ),
      headlineSmall: base.headlineSmall?.copyWith(
        fontSize: 22,
        fontWeight: FontWeight.w700,
        letterSpacing: -0.4,
      ),
      titleLarge: base.titleLarge?.copyWith(
        fontSize: 18,
        fontWeight: FontWeight.w700,
        letterSpacing: -0.2,
      ),
      titleMedium: base.titleMedium?.copyWith(fontWeight: FontWeight.w600),
      bodyMedium: base.bodyMedium?.copyWith(
        color: const Color(0xFFB9B9C6),
        height: 1.45,
      ),
      bodySmall: base.bodySmall?.copyWith(
        color: const Color(0xFF8A8A98),
        height: 1.4,
      ),
      labelLarge: base.labelLarge?.copyWith(
        fontWeight: FontWeight.w600,
        letterSpacing: 0.1,
      ),
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: Colors.transparent,
      elevation: 0,
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: Colors.transparent,
      elevation: 0,
      indicatorColor: accent.withValues(alpha: 0.20),
      labelTextStyle: WidgetStatePropertyAll(
        TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: Colors.white.withValues(alpha: 0.78),
        ),
      ),
      iconTheme: WidgetStateProperty.resolveWith((states) {
        final selected = states.contains(WidgetState.selected);
        return IconThemeData(
          color: selected ? accent : Colors.white.withValues(alpha: 0.58),
        );
      }),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: Colors.white.withValues(alpha: 0.06),
      hintStyle: TextStyle(color: Colors.white.withValues(alpha: 0.34)),
      prefixIconColor: Colors.white.withValues(alpha: 0.44),
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(16),
        borderSide: BorderSide.none,
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(16),
        borderSide: BorderSide(color: Colors.white.withValues(alpha: 0.07)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(16),
        borderSide: BorderSide(color: accent.withValues(alpha: 0.65)),
      ),
    ),
    snackBarTheme: const SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: Color(0xFF1B1B24),
      contentTextStyle: TextStyle(color: Colors.white),
    ),
    dividerTheme: DividerThemeData(
      color: Colors.white.withValues(alpha: 0.07),
      thickness: 1,
    ),
    sliderTheme: SliderThemeData(
      activeTrackColor: accent,
      thumbColor: accent,
      inactiveTrackColor: Colors.white.withValues(alpha: 0.12),
      trackHeight: 3,
    ),
  );
}
