import 'models/UserSaved.dart';

@Service
class UserFormService {
  @Needs
  final UsersClient usersClient;
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;
  String name = '';

  Future<void> save() async {
    final UserSaved saved = await usersClient.save(name);
    applicationEventPublisher.publishEvent(saved, false);
  }
}
