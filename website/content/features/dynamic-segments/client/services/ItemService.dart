@Service
class ItemService {
  String idFrom(String path) { return path.split('/').last; }
}
