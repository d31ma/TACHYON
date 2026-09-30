@Service
final class ReleaseService {
    var release: String = ""
    let applicationEventPublisher: ApplicationEventPublisher

    func announce() {
        applicationEventPublisher.publishEvent(ReleaseChanged(release: "TACHYON"), true)
    }

    @OnEvent({"replay":true})
    func receive(_ changed: ReleaseChanged) { release = changed.release }
}
