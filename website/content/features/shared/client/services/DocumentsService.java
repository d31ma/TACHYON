import java.util.List;

@Service
@NeedsAll
class DocumentsService {
    List<String> order;

    int guides() {
        return order.size();
    }
}
