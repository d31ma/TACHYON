import models.Topic;

@Service
class LinkService {
    Topic topic = new Topic("routing", "Routing");
    String state = "ready";
    String message = "Everything is up to date.";
}
