@Service
@NeedsAll
final class DocumentsService {
  let order: [String]

  func guides() -> Int { order.count }
}
