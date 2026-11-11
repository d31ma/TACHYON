import models.ClaimConflict

@Test(OrdersTestSetup::class, {"partition":"tac-http-verbs"})
class OrderServiceTest {
    @Fake
    var ordersClient: OrdersClient? = null

    suspend fun testPutSendsThePathTheHeaderAndTheBody() {
        val sent = construct(OrderService::class).replace()
        Assert.equals("PUT", sent.method)
        Assert.equals("/api/orders/a-7", sent.path)
        Assert.equals("north", sent.header)
        Assert.equals("hammer", sent.item)
    }

    suspend fun testPatchSendsThePathTheHeaderAndTheBody() {
        val sent = construct(OrderService::class).change()
        Assert.equals("PATCH", sent.method)
        Assert.equals("/api/orders/a-7", sent.path)
        Assert.equals("south", sent.header)
        Assert.equals("nails", sent.item)
    }

    suspend fun testDeleteSendsThePathTheHeaderAndTheRequestParamWithoutABody() {
        val sent = construct(OrderService::class).remove()
        Assert.equals("DELETE", sent.method)
        Assert.equals("/api/orders/a-7", sent.path)
        Assert.equals("duplicate", sent.header)
        Assert.equals("none", sent.item)
        Assert.equals("yes", sent.param)
    }

    suspend fun testErrorBodyIsReadIntoAModel() {
        val service = construct(OrderService::class)
        val error = runCatching { service.claim() }.exceptionOrNull() as Behaviors.Failure
        val conflict = Behaviors.errorBody<ClaimConflict>(error)
        Assert.equals("HTTP_STATUS", error.code)
        Assert.equals(409, error.status)
        Assert.equals("The HTTP request failed.", error.message)
        Assert.equals("taken", conflict?.reason)
        Assert.equals("south", conflict?.holder)
    }
}
