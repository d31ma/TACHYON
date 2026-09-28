[Service]
class BillingService {
  [Needs]
  BillingStore _repository;
  [Needs]
  BillingHelper _helper;

  public int total() {
    int subtotal = _repository.subtotal();
    return _helper.settle(subtotal);
  }
}
