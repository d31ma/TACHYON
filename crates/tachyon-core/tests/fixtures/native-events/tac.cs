[Lifetime({"value":"prototype"})]
public class OwnerProbeService {
    public int Count = 0;
    public string Log = "";
    public int Deaths = Convert.ToInt32(TacStore.Session("eventDeaths", 0));
    private readonly ApplicationEventPublisher applicationEventPublisher;
    [OnStart]
    public void Start() {
        Count = 1;
        Send();
    }
    [OnStop]
    public void Stop() {
        TacStore.SetSession("eventDeaths", Deaths + 1);
    }
    public int Send() {
        applicationEventPublisher.PublishEvent(new Models.Pulse(5), true);
        return 5;
    }
    [OnEvent]
    public void Receive(Models.Pulse value) {
        Count += value.Amount;
        Relay();
    }
    public int Relay() {
        applicationEventPublisher.PublishEvent(new Models.Relay(3));
        return 3;
    }
    [OnEvent]
    public void ReceiveRelay(Models.Relay value) {
        Count += value.Amount;
    }
    public int Cycle() {
        applicationEventPublisher.PublishEvent(new Models.Cycle(1));
        return 1;
    }
    [OnEvent]
    public void ReceiveCycle(Models.Cycle value) {
        Count += value.Amount;
        Cycle();
    }
    public object Isolated() {
        applicationEventPublisher.PublishEvent(new Models.Isolation(1));
        return new Dictionary<string, object> { ["value"] = 1 };
    }
    [OnEvent]
    public void AMutation(Models.Isolation value) {
        var changed = value with { Value = 99 };
        Count += changed.Value - 99;
    }
    [OnEvent]
    public void BOriginal(Models.Isolation value) {
        Count += value.Value;
    }
    public int PublishOrdered() {
        applicationEventPublisher.PublishEvent(new Models.Ordered("paid", 12));
        return 1;
    }
    public int PublishOther() {
        applicationEventPublisher.PublishEvent(new Models.Other("paid", 12));
        return 1;
    }
    [Order(-1)]
    [OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    public Models.Echo Ordinal(Models.Ordered value) {
        Log += "1";
        return new Models.Echo(7);
    }
    [OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    public Models.Echo OtherOrdinal(Models.Other value) {
        Log += "1";
        return new Models.Echo(7);
    }
    [OnEvent]
    public void Second(Models.Ordered value) {
        Log += "2";
    }
    [OnEvent]
    public void Failing(Models.Ordered value) {
        throw new InvalidOperationException("fixture");
    }
    [OnEvent({"condition":"value.total >= 100"})]
    public void Skipped(Models.Ordered value) {
        Log += "S";
    }
    [Order(1)]
    [InBackground]
    [OnEvent]
    public Models.Echo DeferredListener(Models.Ordered value) {
        Log += "A";
        return new Models.Echo(1000);
    }
    [Order(2)]
    [OnEvent]
    public void Third(Models.Ordered value) {
        Log += "3";
    }
    [OnEvent]
    public void OnEcho(Models.Echo value) {
        Log += "E";
        Count += value.Amount;
    }
}
