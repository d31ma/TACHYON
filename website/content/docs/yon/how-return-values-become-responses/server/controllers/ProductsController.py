from http import HTTPStatus


@request_route("/products")
@public
class ProductsController:
    @get_route
    def get_products(self) -> object:
        body = "<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>"
        return (HTTPStatus.OK, {"content-type": "text/html; charset=utf-8"}, body)
