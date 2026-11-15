use crate::models::Identity;

#[RequestRoute("/me")]
#[SignedIn]
struct MeController {}
impl MeController {
    fn new() -> Self {
        Self {}
    }

    #[GetRoute]
    fn get_me(&self, #[FromIdentity] caller: Identity) -> String {
        caller.user
    }
}
