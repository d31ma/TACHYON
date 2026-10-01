struct NativeUiStyleScript {
    static let source = #"""
__STYLE_TRANSACTION_JS__
return await NativeUiStyleInvocation.run(operation, ticket, styles, deadline)
"""#
}
