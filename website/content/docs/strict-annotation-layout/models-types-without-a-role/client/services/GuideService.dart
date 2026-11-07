import 'models/Guide.dart';

@Service
class GuideService {
  static const Guide _current = Guide('Models hold types', 3);

  String title = _current.title;
  int pages = _current.pages;
}
