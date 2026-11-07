import 'models/Item.dart';
import 'models/ItemList.dart';

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute
  ItemList getItems() => const ItemList([]);

  @PostRoute
  @ResponseStatus(201)
  Item postItem(@FromBody Item item) => item;
}
