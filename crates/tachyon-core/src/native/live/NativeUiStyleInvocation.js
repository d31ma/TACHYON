class NativeUiStyleInvocation {
  static async run(operation, ticket, styles, deadline) {
    const controller = globalThis.__tachyonNativeStyles ??= new NativeUiStyleController()
    return await controller.run(operation, ticket, styles, deadline)
  }
}
