class NativeUiStyleOriginal {
  constructor(node) {
    this.node = node
    this.attributes = new NativeUiStyleAttributes(node)
    this.parent = node.parentNode
    this.next = node.nextSibling
  }

  unchanged(candidate) {
    return this.boundary(candidate) && this.attributes.matches(this.node)
  }

  boundary(candidate) {
    return this.node.isConnected && this.node.parentNode === this.parent
      && this.node.nextSibling === candidate && candidate.nextSibling === this.next
  }
}
