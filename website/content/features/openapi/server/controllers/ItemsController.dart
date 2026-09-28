import 'models/Item.dart';

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute("/{id}")
  Item getItem(@FromPath int id) => Item('anvil', id);

  @PatchRoute("/{id}")
  Item patchItem(@FromPath int id, @FromBody Item item) => Item(item.name, id);
}
