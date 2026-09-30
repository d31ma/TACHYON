@Service
class ReportService {
  @Needs()
  final WindowControls windowControls;
  String problem = '';

  void minimize() => _keepProblem(() => windowControls.minimize());

  void resize() => _keepProblem(() => windowControls.resize(1280, 800));

  void rename() => _keepProblem(() => windowControls.setTitle('Quarterly report'));

  void _keepProblem(void Function() call) {
    try {
      call();
    } on StateError catch (refusal) {
      problem = refusal.message;
    }
  }
}
