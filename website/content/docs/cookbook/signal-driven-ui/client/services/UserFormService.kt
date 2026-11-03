import models.UserSaved

@Service
class UserFormService {
    @Needs
    private val usersClient: UsersClient
    val applicationEventPublisher: ApplicationEventPublisher
    var name: String = ""

    suspend fun save() {
        val saved: UserSaved = usersClient.save(name)
        applicationEventPublisher.publishEvent(saved, false)
    }
}
