#[HTTPClient]
trait ApplicationClient {
    #[GetRequest("/manifest.webmanifest")]
    async fn load(&self) -> BehaviorResult<BehaviorValue>;
}
