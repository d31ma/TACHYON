import 'models/Guide.dart';

@RequestRoute("/guide")
@Public
class GuideController {
  @GetRoute
  Guide getGuide() => const Guide('Models hold types', 3);
}
