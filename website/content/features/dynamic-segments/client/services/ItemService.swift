@Service
final class ItemService {
    func idFrom(_ path: String) -> String {
        String(path.split(separator: "/").last ?? "")
    }
}
