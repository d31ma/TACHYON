@Service
final class ItemService {
    func idFrom(_ path: String) -> String {
        path.split(separator: "/").suffix(1).joined()
    }
}
