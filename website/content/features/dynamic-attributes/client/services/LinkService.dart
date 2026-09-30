import 'models/Topic.dart';

@Service
class LinkService {
  Topic topic = const Topic('routing', 'Routing');
  String state = 'ready';
  String message = 'Everything is up to date.';
}
