[RequestRoute("/status")]
[Public]
class StatusController {
    [GetRoute]
    public string GetStatus() {
        return "ok";
    }
}
