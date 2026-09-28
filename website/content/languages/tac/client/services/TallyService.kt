@Service
class TallyService {
  var count: Int = 6
  val runtime: String = "Kotlin, compiled to WebAssembly or into the APK"

  fun doubled(): Int = count * 2
}