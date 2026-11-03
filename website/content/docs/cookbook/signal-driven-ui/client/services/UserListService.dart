import 'models/UserSaved.dart';

@Service
class UserListService {
  @Needs
  final UsersClient usersClient;
  List<String> names = [];

  @OnEvent
  Future<void> userSaved(UserSaved saved) async {
    names = _names(await usersClient.all());
  }

  List<String> _names(List<UserSaved> users) => [for (final user in users) user.name];
}
