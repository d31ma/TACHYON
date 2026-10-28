#[Delegate]
#[Lifetime("singleton")]
class LedgerAmountsDelegate
  # @return [Integer]
  def subtotal
    40
  end

  # @return [Integer]
  def surcharge
    2
  end
end
