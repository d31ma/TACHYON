use crate::models::ProgramState;

#[HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})]
trait PageProgramClient {
    #[GetRequest("/read")]
    async fn read(&self) -> BehaviorResult<ProgramState>;

    #[PostRequest("/increment")]
    async fn increment(&self) -> BehaviorResult<ProgramState>;
}
