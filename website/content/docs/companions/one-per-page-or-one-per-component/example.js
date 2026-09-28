@Service
@Lifetime({"value":"prototype"})
export default class SnippetService {
  constructor({ files = [] } = {}) {
    this.files = files
  }

  shown() { return this.files }
}