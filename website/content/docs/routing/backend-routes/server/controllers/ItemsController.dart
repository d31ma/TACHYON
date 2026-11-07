import 'models/Item.dart';

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute("/{id}")
  Item getItem(@FromPath String id) => Item(id);

  @PostRoute
  @ResponseStatus(201)
  Item postItem(@FromBody Item item) => item;
}
