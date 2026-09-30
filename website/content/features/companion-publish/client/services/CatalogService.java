import models.CatalogCount;

class CatalogService {
    int count = 0;
    ApplicationEventPublisher applicationEventPublisher;

    @OnStart
    void initialize() { announce(); }

    private void announce() { applicationEventPublisher.publishEvent(new CatalogCount(5)); }

    @OnEvent
    void changed(CatalogCount value) { count = value.count(); }

    @OnStop
    void cleanup() { count = 0; }
}
