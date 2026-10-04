@Service
final class UserListService {
    var names: [String] = []

    @OnEvent
    func userSaved(_ saved: UserSaved) { names.append(saved.name) }
}
