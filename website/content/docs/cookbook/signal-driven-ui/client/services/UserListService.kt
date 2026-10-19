import models.UserSaved

@Service
class UserListService {
    @Needs
    private val usersClient: UsersClient
    var names: List<String> = emptyList()

    @OnEvent
    fun userSaved(saved: UserSaved) { names = namesOf(usersClient.all()) }

    private fun namesOf(users: Any?): List<String> =
        (users as? List<*>).orEmpty().mapNotNull { (it as? Map<*, *>)?.get("name")?.toString() }
}
