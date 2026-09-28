import 'models/Money.dart';

@RequestRoute("/money")
@Public
class MoneyController {
  @Needs
  final MoneyService money;

  @GetRoute
  Money getMoney() => Money(money.amount());
}
