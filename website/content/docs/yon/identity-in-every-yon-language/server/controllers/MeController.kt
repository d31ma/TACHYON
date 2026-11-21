import models.Identity

@RequestRoute("/me")
@SignedIn
class MeController {
    @GetRoute
    fun getMe(@FromIdentity caller: Identity): String {
        return caller.user
    }
}
