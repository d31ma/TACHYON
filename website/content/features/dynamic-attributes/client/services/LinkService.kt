import models.Topic

@Service
class LinkService {
  var topic: Topic = Topic("routing", "Routing")
  var state: String = "ready"
  var message: String = "Everything is up to date."
}
