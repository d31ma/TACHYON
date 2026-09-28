@Service
final class TallyService {
  var count: Int = 6
  let runtime = "Swift, compiled into the app"

  func doubled() -> Int { count * 2 }
}