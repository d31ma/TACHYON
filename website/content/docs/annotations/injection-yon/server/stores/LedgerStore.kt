@Store
@Lifetime("singleton")
class LedgerStore {
  fun subtotal(): Int = 40
  fun surcharge(): Int = 2
}
