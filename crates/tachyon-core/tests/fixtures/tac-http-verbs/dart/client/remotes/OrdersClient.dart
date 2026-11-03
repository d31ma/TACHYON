@HTTPClient({"path":"/api","adapter":"orders","setup":"OrdersSetup"})
abstract interface class OrdersClient {
  @PutRequest("/orders/{id}")
  Future<SentRequest> replace(@PathVariable("id") String id, @RequestBody Map<String, Object?> order, @RequestHeader("X-Tenant") String tenant);

  @PatchRequest("/orders/{id}")
  Future<SentRequest> change(@PathVariable("id") String id, @RequestBody Map<String, Object?> change, @RequestHeader("X-Tenant") String tenant);

  @DeleteRequest("/orders/{id}")
  Future<SentRequest> remove(@PathVariable("id") String id, @RequestHeader("X-Reason") String reason);
}
