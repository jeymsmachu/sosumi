import 'package:flutter/material.dart';

/// Shows a short, consistent placeholder message for scaffold-only controls.
void showScaffoldNote(BuildContext context, String feature) {
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(content: Text('$feature is not wired yet.')));
}
