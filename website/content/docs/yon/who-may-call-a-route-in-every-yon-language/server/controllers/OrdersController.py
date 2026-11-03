@request_route("/orders")
@signed_in
class OrdersController:
    @get_route
    @public
    def get_orders(self) -> list[str]:
        return ["lamp", "desk"]

    @post_route
    def post_order(self) -> list[str]:
        return ["chair"]

    @delete_route
    @allow(Admin)
    def delete_orders(self) -> list[str]:
        return []
