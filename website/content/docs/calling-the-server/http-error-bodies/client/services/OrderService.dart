@Service
class OrderService {
  @Needs
  final OrderClient orderClient;
  int status = 0;
  String title = '';
  int mistakes = 0;

  Future<void> send() async {
    try {
      await orderClient.send({'name': '', 'level': 12});
    } on BehaviorError catch (error) {
      final problem = BehaviorErrors.body(error, tachyonReadModelProblem);
      status = BehaviorErrors.status(error);
      title = problem?.title ?? '';
      mistakes = problem?.errors.length ?? 0;
    }
  }
}
