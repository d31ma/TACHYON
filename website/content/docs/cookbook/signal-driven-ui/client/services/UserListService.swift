@Service
final class UserListService {
    @Needs
    private let usersClient: UsersClient
    var names: [String] = []

    @OnEvent
    func userSaved(_ saved: UserSaved) async throws {
        names = Self.namesOf(try await usersClient.all())
    }

    private static func namesOf(_ users: BehaviorJSON) -> [String] {
        guard case .array(let items) = users else { return [] }
        return items.compactMap { try? $0.get("name").text }
    }
}
