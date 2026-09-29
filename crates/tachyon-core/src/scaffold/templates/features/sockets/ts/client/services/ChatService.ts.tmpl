import type { ChatMessage } from '/models/ChatMessage.ts';

@Service
export default class ChatService {
  @Needs
  readonly chatClient!: ChatClient;
  messages: ChatMessage[] = []

  @OnEvent
  addMessage(message: ChatMessage): void {
    this.messages = [...this.messages.slice(-49), message]
  }

  sayHello(): Promise<void> {
    return this.chatClient.send({ text: 'Hello from a tab' })
  }
}
