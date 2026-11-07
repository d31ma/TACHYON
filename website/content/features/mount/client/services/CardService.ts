@Service
export default class CardService {
  name: string = 'Anvil'
  status: string = 'waiting'

  @OnStart
  activate(): void { this.status = 'active' }
}
