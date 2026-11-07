#[Delegate]
class BillingDelegate
  SURCHARGE = 2

  # @param subtotal [Integer]
  # @return [Integer]
  def settle(subtotal)
    subtotal + SURCHARGE
  end
end
