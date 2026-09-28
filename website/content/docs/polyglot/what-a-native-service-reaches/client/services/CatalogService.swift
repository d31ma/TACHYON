class CatalogService {
    var count: Int = 0
    let applicationEventPublisher: ApplicationEventPublisher

    @OnStart
    func initialize() { announce() }

    private func announce() { applicationEventPublisher.publishEvent(CatalogCount(count: 5)) }

    @OnEvent
    func changed(_ value: CatalogCount) { count = value.count }

    @OnStop
    func cleanup() { count = 0 }
}
