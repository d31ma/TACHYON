@Service
@NeedsAll
class DocumentsService {
  late final List<String> order;

  int guides() => order.length;
}
