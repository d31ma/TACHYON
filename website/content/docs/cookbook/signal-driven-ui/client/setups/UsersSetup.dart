@Setup
class UsersSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    final saved = <Object?>[];
    return BehaviorAdapters(
      http: {
        'users': BehaviorHttpAdapter(exchange: (request, running) async {
          if (BehaviorJson.get(request, 'method') == 'GET') return saved;
          final user = {'name': BehaviorJson.get(request, 'body')};
          saved.add(user);
          return user;
        }),
      },
    );
  }
}
