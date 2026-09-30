@Service
class ItemService {
    fun idFrom(path: String): String = path.substringAfterLast('/')
}
