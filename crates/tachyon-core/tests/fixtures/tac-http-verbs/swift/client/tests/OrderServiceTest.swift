@Test(OrdersTestSetup, {"partition":"tac-http-verbs"})
final class OrderServiceTest {
    @Fake
    var ordersClient: OrdersClient? = nil

    func testPutSendsThePathTheHeaderAndTheBody() async throws {
        let orders: OrderService = try construct()
        let sent = try await orders.replace()
        try Assert.equals("PUT", sent.method)
        try Assert.equals("/api/orders/a-7", sent.path)
        try Assert.equals("north", sent.header)
        try Assert.equals("hammer", sent.item)
    }

    func testPatchSendsThePathTheHeaderAndTheBody() async throws {
        let orders: OrderService = try construct()
        let sent = try await orders.change()
        try Assert.equals("PATCH", sent.method)
        try Assert.equals("/api/orders/a-7", sent.path)
        try Assert.equals("south", sent.header)
        try Assert.equals("nails", sent.item)
    }

    func testDeleteSendsThePathAndTheHeaderWithoutABody() async throws {
        let orders: OrderService = try construct()
        let sent = try await orders.remove()
        try Assert.equals("DELETE", sent.method)
        try Assert.equals("/api/orders/a-7", sent.path)
        try Assert.equals("duplicate", sent.header)
        try Assert.equals("none", sent.item)
    }
}
