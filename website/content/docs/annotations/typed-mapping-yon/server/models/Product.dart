import 'Detail.dart';
class Product {
final int id;
final String name;
final Detail detail;
final List<Detail> items;
final String secret;
const Product(this.id, this.name, this.detail, this.items, this.secret);
}
