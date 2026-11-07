@Service
export default class CardService {
  name = 'Anvil'
  status = 'waiting'

  @OnStart
  activate() { this.status = 'active' }
}
