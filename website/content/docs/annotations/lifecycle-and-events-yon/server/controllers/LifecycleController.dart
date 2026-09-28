import 'models/Lifecycle.dart';

@RequestRoute("/lifecycle")
@Public
class LifecycleController {
  @Needs
  final AuditService audit;
  @Needs
  final CheckoutService checkout;

  @GetRoute
  Lifecycle getLifecycle() {
    return Lifecycle(audit.log, audit.settled);
  }
}
