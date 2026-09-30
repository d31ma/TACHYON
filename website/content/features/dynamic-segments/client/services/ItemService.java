@Service
class ItemService {
    String idFrom(String path) {
        return path.substring(path.lastIndexOf('/') + 1);
    }
}
