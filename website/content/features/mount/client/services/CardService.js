@Service
export default class CardService {
  name = 'Anvil'

  @OnStart
  initialize() { document.title = this.name }
}
