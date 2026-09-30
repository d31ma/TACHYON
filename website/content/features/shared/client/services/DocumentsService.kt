@Service
@NeedsAll
class DocumentsService {
  val order: List<String>

  fun guides(): Int = order.size
}
