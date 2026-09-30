@Service
class RuntimeNoteService {
  final String runtime = 'Dart, compiled to WebAssembly';

  bool showsNotes(bool development) { return development; }
}
