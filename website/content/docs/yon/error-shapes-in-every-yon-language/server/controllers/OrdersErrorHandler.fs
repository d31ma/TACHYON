open Models

[<ErrorHandler>]
type OrdersErrorHandler() =
    [<HandlesError>]
    member this.SignInNeeded(failure: NotSignedIn) : Problem =
        { Code = "not_signed_in"; Message = "[MS0001]: Sign in again." }

    [<HandlesError>]
    member this.NotAllowed(failure: RoleMissing) : Problem =
        { Code = "forbidden"; Message = "[MS0002]: You may not do this." }

    [<HandlesError>]
    member this.IdentityDown(failure: IdentityCheckFailed) : Problem =
        { Code = "identity_failed"; Message = "[MS0003]: Try again later." }

    [<HandlesError>]
    member this.BadRequest(failure: RequestRejected) : Problem =
        { Code = "invalid_request"; Message = "[MS0004]: " + failure.Part + "." + failure.Field }

    [<HandlesError>]
    member this.TooLarge(failure: BodyTooLarge) : Problem =
        { Code = "too_large"; Message = "[MS0005]: " + string failure.LimitBytes }

    [<HandlesError>]
    member this.BrokenRules(failure: CheckRejected) : Problem =
        { Code = "invalid_input"; Message = "[MS0006]: " + this.Rules(failure) }

    [<HandlesError>]
    member this.Unexpected(failure: UnexpectedError) : Problem =
        { Code = "unexpected"; Message = "[MS0000]: Something went wrong." }

    member private this.Rules(failure: CheckRejected) : string =
        failure.Violations
        |> Seq.map (fun violation -> violation.Field + ":" + violation.Rule)
        |> String.concat ","
