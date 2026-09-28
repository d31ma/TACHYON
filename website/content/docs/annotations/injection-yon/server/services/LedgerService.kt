@Service
class LedgerService {
  @Needs
  val repository: LedgerStore
  fun subtotal(): Int = repository.subtotal()
}
