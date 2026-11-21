import models.Identity

@RequestRoute("/me")
@SignedIn
class MeController {
    @GetRoute
    String getMe(@FromIdentity Identity caller) {
        return caller.user()
    }
}
