@HTTPClient("/api")
abstract interface class OrderClient {
  @PostRequest("/orders")
  Future<OrderInput> send(@RequestBody Map<String, Object?> order);
}
