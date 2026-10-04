import models.UserSaved

@Service
class UserListService {
    var names: List<String> = emptyList()

    @OnEvent
    fun userSaved(saved: UserSaved) { names = names + saved.name }
}
