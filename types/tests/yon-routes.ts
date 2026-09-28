interface Order {
  id: string;
  total: number;
}

@RequestRoute('/orders')
class OrdersController {
  @GetRoute
  getOrders(@FromQuery('page') page: number): Order[] {
    return [{ id: String(page), total: 1 }];
  }

  @GetRoute('/{id}')
  getOrderById(@FromPath id: string, @FromHeader('x-tenant') tenant: string, @FromCookie('theme') theme: string): Order {
    return { id: `${id}-${tenant}-${theme}`, total: 2 };
  }

  @PostRoute
  @ResponseStatus(201)
  postOrder(@FromBody order: Order): Order {
    return order;
  }

  @PutRoute('/{id}')
  putOrder(@FromPath('id') id: string, @FromBody() order: Order): Order {
    return { ...order, id };
  }

  @PatchRoute('/{id}')
  patchOrder(@FromPath id: string): Order {
    return { id, total: 3 };
  }

  @DeleteRoute('/{id}')
  deleteOrder(@FromPath id: string): { removed: string } {
    return { removed: id };
  }
}

@HTTPClient('/api/orders')
class OrdersClient {
  @GetRequest('/{id}')
  async getOrder(@PathVariable id: string, @RequestParam('view') view: string): Promise<BehaviorJsonValue> {
    return { id, view };
  }

  @PostRequest
  async postOrder(@RequestBody order: BehaviorJsonObject, @RequestHeader('x-tenant') tenant: string): Promise<BehaviorJsonValue> {
    return { order, tenant };
  }

  @PutRequest('/{id}')
  async putOrder(@PathVariable('id') id: string): Promise<BehaviorJsonValue> {
    return id;
  }

  @PatchRequest('/{id}')
  async patchOrder(@PathVariable id: string): Promise<BehaviorJsonValue> {
    return id;
  }

  @DeleteRequest('/{id}')
  async deleteOrder(@PathVariable id: string): Promise<BehaviorJsonValue> {
    return id;
  }
}

@HTTPClient({ path: '/api', adapter: 'catalog', cache: 'no-store' })
class CatalogClient {}

interface OrderTick { id: string; level: number }

@HTTPClient('/api/orders')
class OrderTicksClient {
  @StreamRequest('/ticks')
  streamTicks(tick: OrderTick) {}

  @StreamRequest
  async streamMore(tick: OrderTick): Promise<void> {}

  // @ts-expect-error StreamRequest methods take one message parameter.
  @StreamRequest
  streamTwo(tick: OrderTick, other: OrderTick) {}

  // @ts-expect-error A stream message is a model object, not text.
  @StreamRequest
  streamText(text: string) {}
}

interface ChatMessage { text: string }

@SocketRoute('/chat')
class ChatController {
  count = 0;

  @OnOpen
  onOpen(session: SocketSession) {}

  @OnMessage
  async onMessage(session: SocketSession, message: ChatMessage): Promise<void> {
    this.count += 1;
    for (const peer of await session.openSessions()) await peer.send(message);
  }

  @OnClose
  onClose() {}

  @OnError
  onError(session: SocketSession, error: unknown) {}

  // @ts-expect-error An OnMessage method takes the session and the message.
  @OnMessage
  onlyMessage(message: ChatMessage) {}
}

@HTTPClient('/api')
class ChatClient {
  declare send: SocketSend<ChatMessage>;

  @SocketRequest('/chat')
  receive(message: ChatMessage) {}

  // @ts-expect-error SocketRequest methods take one message parameter.
  @SocketRequest
  receiveTwo(message: ChatMessage, other: ChatMessage) {}
}

export const sockets = [ChatController, ChatClient];

class OrderNotFound extends Error {}

@ErrorHandler
class OrdersErrorHandler {
  @HandlesError
  @ResponseStatus(404)
  missing(error: OrderNotFound): { code: string } {
    return { code: error.name };
  }

  @HandlesError()
  failure(error: Error, request: { path: string }): { code: string } {
    return { code: `${error.name}:${request.path}` };
  }

  // @ts-expect-error HandlesError requires an Error-compatible parameter.
  @HandlesError
  notAnError(value: number): number { return value; }

  // @ts-expect-error HandlesError accepts only an error and optional request.
  @HandlesError
  tooMany(error: Error, request: object, extra: string): string { return extra; }
}

// @ts-expect-error HandlesError infers the type and takes no string argument.
HandlesError('OrderNotFound');
// @ts-expect-error ErrorHandler is a bare class marker, not a path declaration.
ErrorHandler('/errors');

// @ts-expect-error Client was replaced by HTTPClient.
Client();
// @ts-expect-error WebCall was replaced by request annotations.
WebCall({ client: 'catalog', method: 'GET', path: '/products' });
// @ts-expect-error HTTPExchange was replaced by HTTPClient.
HTTPExchange('/orders');
// @ts-expect-error GetExchange was replaced by GetRequest.
GetExchange('/orders');
// @ts-expect-error RequestMapping was replaced by RequestRoute.
RequestMapping('/orders');
// @ts-expect-error GetMapping was replaced by GetRoute.
GetMapping('/orders');

export { OrdersController, OrdersClient, CatalogClient, OrderTicksClient, OrdersErrorHandler };
