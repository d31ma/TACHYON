import 'models/ProgramState.dart';

@Service
class ReaderService {
  static const _noteId = 'note';
  static const _textField = 'text';

  @Needs
  final PageProgramClient pageProgramClient;
  @Needs
  final ProgramNoteStore programNoteStore;
  String identity = '';
  int count = 0;
  String recalled = '';

  Future<void> inspect() async {
    _show(await pageProgramClient.read());
  }

  Future<void> increment() async {
    _show(await pageProgramClient.increment());
  }

  Future<void> remember(String text) async {
    await programNoteStore.save({'id': _noteId, _textField: text});
  }

  Future<void> recall() async {
    recalled = _textOf(await programNoteStore.findById(_noteId));
  }

  void _show(ProgramState state) {
    identity = state.identity;
    count = state.count;
  }

  String _textOf(Object? note) {
    if (note == null) return '';
    return BehaviorJson.text(BehaviorJson.get(note, _textField));
  }
}
