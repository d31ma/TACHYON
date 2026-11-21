using Models;

[RequestRoute("/me")]
[SignedIn]
class MeController {
    [GetRoute]
    public string GetMe([FromIdentity] Identity caller) {
        return caller.User;
    }
}
