@Service
final class UserFormService {
    private let usersClient: UsersClient = UsersClient()
    let applicationEventPublisher: ApplicationEventPublisher
    var name: String = ""

    func save() async throws {
        let answer = try await usersClient.save(name: .string(name))
        applicationEventPublisher.publishEvent(UserSaved(name: try answer.get("name").text), false)
    }
}
