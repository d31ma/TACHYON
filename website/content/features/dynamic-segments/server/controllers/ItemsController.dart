import 'models/Item.dart';

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute("/{id}")
  Item getItem(@FromPath int id) => Item(id);
}
