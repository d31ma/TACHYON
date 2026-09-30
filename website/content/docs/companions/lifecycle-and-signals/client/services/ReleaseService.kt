import models.ReleaseChanged

@Service
class ReleaseService {
    var release: String = ""
    val applicationEventPublisher: ApplicationEventPublisher

    fun announce() {
        applicationEventPublisher.publishEvent(ReleaseChanged("TACHYON"), true)
    }

    @OnEvent({"replay":true})
    fun receive(changed: ReleaseChanged) { release = changed.release }
}
