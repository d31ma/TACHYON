@Service
final class LinkService {
  var topic: Topic = Topic(slug: "routing", title: "Routing")
  var state: String = "ready"
  var message: String = "Everything is up to date."
}
