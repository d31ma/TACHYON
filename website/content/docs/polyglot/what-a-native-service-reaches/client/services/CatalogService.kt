import models.CatalogCount

class CatalogService {
    var count: Int = 0
    val applicationEventPublisher: ApplicationEventPublisher

    @OnStart
    fun initialize() { announce() }

    private fun announce() { applicationEventPublisher.publishEvent(CatalogCount(5)) }

    @OnEvent
    fun changed(value: CatalogCount) { count = value.count }

    @OnStop
    fun cleanup() { count = 0 }
}
