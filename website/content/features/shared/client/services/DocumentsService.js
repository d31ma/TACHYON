import documents from '/shared/data/docs.json' with { type: 'json' }

@Service
export default class DocumentsService {
  /** @type {number} */
  guides = documents.order.length
}
