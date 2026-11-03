@Service
class CatalogService {
  @Needs
  final CatalogClient catalog;
  List<String> products = [];
  String error = '';

  Future<void> load() async {
    await catalog.all().then(_show, onError: (_) => _showFailure());
  }

  void _show(Object? answer) {
    if (answer is! List) return _showFailure();
    products = answer.map((item) => '$item').toList();
    error = '';
  }

  void _showFailure() {
    error = 'Products could not be loaded. Please retry.';
  }
}
