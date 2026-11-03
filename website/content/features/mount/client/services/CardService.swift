@Service
final class CardService {
  let name: String = "Anvil"
  var status: String = "waiting"

  @OnStart
  func activate() { status = "active" }
}
