import models.CountChanged

@Service
class CounterService {
    var count: Int = 0
    val applicationEventPublisher: ApplicationEventPublisher

    fun add() {
        count += 1
        applicationEventPublisher.publishEvent(CountChanged(count))
    }
}
