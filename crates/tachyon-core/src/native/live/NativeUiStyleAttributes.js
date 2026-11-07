class NativeUiStyleAttributes {
  constructor(node) {
    this.values = node.getAttributeNames().sort().map(name => [name, node.getAttribute(name)])
    this.media = node.media
    this.disabled = node.disabled
  }

  matches(node) {
    const current = new NativeUiStyleAttributes(node)
    return JSON.stringify(this) === JSON.stringify(current)
  }

  restore(node) {
    this._disabled(node)
    this._attribute(node, 'disabled')
    this._attribute(node, 'media')
  }

  _disabled(node) {
    if (node.disabled !== this.disabled) node.disabled = this.disabled
  }

  _attribute(node, name) {
    const entry = this.values.find(([key]) => key === name)
    if (entry) node.setAttribute(name, entry[1])
    else node.removeAttribute(name)
  }
}
