using Models;

public class CatalogService {
    public int Count = 0;
    private readonly ApplicationEventPublisher applicationEventPublisher;

    [OnStart]
    public void Initialize() { Announce(); }

    private void Announce() { applicationEventPublisher.PublishEvent(new CatalogCount(5)); }

    [OnEvent]
    public void Changed(CatalogCount value) { Count = value.Count; }

    [OnStop]
    public void Cleanup() { Count = 0; }
}
