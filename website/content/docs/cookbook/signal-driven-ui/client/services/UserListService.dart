import 'models/UserSaved.dart';

@Service
class UserListService {
  List<String> names = [];

  @OnEvent
  void userSaved(UserSaved saved) {
    names = [...names, saved.name];
  }
}
