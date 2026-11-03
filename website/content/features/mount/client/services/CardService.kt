@Service
class CardService {
  val name: String = "Anvil"
  var status: String = "waiting"

  @OnStart
  fun activate() { status = "active" }
}
