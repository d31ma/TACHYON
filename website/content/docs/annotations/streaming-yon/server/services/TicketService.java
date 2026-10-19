import java.util.List;

@Service
class TicketService {
    List<Integer> levels() { return List.of(1, 2, 3); }
}
