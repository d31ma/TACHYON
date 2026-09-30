import 'package:shelf/shelf.dart';

@RequestRoute("/products")
@Public
class ProductsController {
  @GetRoute
  Response getProducts() => Response(
        200,
        body: '<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>',
        headers: {'content-type': 'text/html; charset=utf-8'},
      );
}
