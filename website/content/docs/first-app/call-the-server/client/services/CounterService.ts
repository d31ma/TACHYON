import type { Greeting } from '/models/Greeting.ts'

@Service
export default class CounterService {
  @Needs
  readonly greetingClient!: GreetingClient;
  count = 0
  message = ''

  increment(): void {
    this.count += 1
  }

  async askServer(): Promise<void> {
    const greeting = await this.greetingClient.getGreeting()
    this.message = this.messageOf(greeting)
  }

  private messageOf(greeting: Greeting | void): string {
    return greeting ? greeting.message : ''
  }
}
