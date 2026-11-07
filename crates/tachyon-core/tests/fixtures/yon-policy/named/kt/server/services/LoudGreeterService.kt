@Service
class LoudGreeterService : GreeterService() {
  override fun greet(): String = "HELLO"
}
