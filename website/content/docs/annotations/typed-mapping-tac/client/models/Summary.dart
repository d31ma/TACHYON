import 'ShortDetail.dart';
class Summary {
  final int id;
  final String name;
  final ShortDetail detail;
  final List<ShortDetail> items;
  final String kind;
  const Summary(this.id, this.name, this.detail, this.items, this.kind);
}
