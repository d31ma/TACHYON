@Service
class RenameService {
  @Needs()
  final Focus focus;
  bool editing = false;
  String problem = '';

  void open() {
    editing = true;
    _keepProblem(() => focus.trap('dialog'));
    _keepProblem(() => focus.move('name'));
  }

  void close() {
    editing = false;
    _keepProblem(() => focus.release());
  }

  @OnEvent
  void failed(BrowserCallFailed failed) {
    problem = failed.message;
  }

  void _keepProblem(void Function() call) {
    try {
      call();
    } on StateError catch (refusal) {
      problem = refusal.message;
    }
  }
}
