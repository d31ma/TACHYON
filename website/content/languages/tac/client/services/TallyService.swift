@Service
final class TallyService {
  var count: Int = 6
  let runtime = "Swift, in the app or as WebAssembly"

  func doubled() -> Int { count * 2 }
}