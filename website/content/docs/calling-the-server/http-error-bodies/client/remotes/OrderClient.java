import models.OrderInput;

@HTTPClient("/api")
interface OrderClient {
    @PostRequest("/orders")
    OrderInput send(@RequestBody Object order);
}
