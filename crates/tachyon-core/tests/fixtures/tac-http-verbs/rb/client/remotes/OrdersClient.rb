#[HTTPClient({"path":"/api","adapter":"orders"})]
class OrdersClient
  # @param id [String]
  # @param order [Hash]
  # @param tenant [String]
  # @return [SentRequest]
  #[PutRequest("/orders/{id}")]
  def replace(
    #[PathVariable("id")]
    id,
    #[RequestBody]
    order,
    #[RequestHeader("X-Tenant")]
    tenant
  )
  end

  # @param id [String]
  # @param change [Hash]
  # @param tenant [String]
  # @return [SentRequest]
  #[PatchRequest("/orders/{id}")]
  def change(
    #[PathVariable("id")]
    id,
    #[RequestBody]
    change,
    #[RequestHeader("X-Tenant")]
    tenant
  )
  end

  # @param id [String]
  # @param reason [String]
  # @return [SentRequest]
  #[DeleteRequest("/orders/{id}")]
  def remove(
    #[PathVariable("id")]
    id,
    #[RequestHeader("X-Reason")]
    reason
  )
  end
end
