@Service
class CounterService {
    var count: Int = 0
    let applicationEventPublisher: ApplicationEventPublisher

    func add() {
        count += 1
        applicationEventPublisher.publishEvent(CountChanged(count: count))
    }
}
