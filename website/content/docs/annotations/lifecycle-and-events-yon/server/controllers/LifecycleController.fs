open Models

[<RequestRoute("/lifecycle")>]
[<Public>]
type LifecycleController(auditService: AuditService, checkoutService: CheckoutService) =
    [<GetRoute>]
    member this.GetLifecycle() : Lifecycle =
        { Log = List.ofSeq auditService.Log; Settled = auditService.Settled }
