@Service
@NeedsAll
export default class DocumentsService {
  order!: string[]

  guides(): number {
    return this.order.length
  }
}
