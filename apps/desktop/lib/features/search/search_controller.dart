import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/engine.dart';
import '../../rust/api.dart';

/// The current search, and its results.
class SearchState {
  const SearchState({this.query = '', this.results, this.loading = false, this.error});

  final String query;
  final SearchResultsDto? results;
  final bool loading;
  final String? error;

  bool get hasResults {
    final r = results;
    if (r == null) return false;
    return r.tracks.isNotEmpty ||
        r.albums.isNotEmpty ||
        r.artists.isNotEmpty ||
        r.playlists.isNotEmpty;
  }
}

/// Runs searches against the Rust core.
class SearchController extends StateNotifier<SearchState> {
  SearchController(this._engine) : super(const SearchState());

  final Engine _engine;
  int _generation = 0;

  /// Runs a search, discarding the result of any earlier one still in flight.
  ///
  /// A generation counter rather than a timer: the last query typed must win,
  /// and a slow response for an earlier query must never overwrite it.
  Future<void> search(String query) async {
    final trimmed = query.trim();
    if (trimmed.isEmpty) {
      state = const SearchState();
      return;
    }

    final generation = ++_generation;
    state = SearchState(query: trimmed, results: state.results, loading: true);

    try {
      final results = await _engine.search(query: trimmed);
      if (generation != _generation) return;
      state = SearchState(query: trimmed, results: results);
    } on Exception catch (error) {
      if (generation != _generation) return;
      state = SearchState(query: trimmed, error: '$error');
    }
  }

  /// Clears the current search.
  void clear() {
    _generation++;
    state = const SearchState();
  }
}

/// The search controller.
final searchProvider = StateNotifierProvider<SearchController, SearchState>((ref) {
  final engine = ref.watch(engineProvider).requireValue;
  return SearchController(engine);
});
