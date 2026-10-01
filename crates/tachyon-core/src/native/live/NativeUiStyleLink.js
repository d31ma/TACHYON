class NativeUiStyleLink {
  constructor(node, href) {
    this.original = new NativeUiStyleOriginal(node)
    this.candidate = node.cloneNode(false)
    this.candidate.removeAttribute('id')
    this.candidate.href = href
    this.candidate.media = 'not all'
    this.candidate.disabled = false
    this.pending = new NativeUiStyleAttributes(this.candidate)
    this.cancelLoad = null
  }

  async prepare(deadline) {
    const loaded = this._load(deadline)
    this.original.parent.insertBefore(this.candidate, this.original.next)
    await loaded
  }

  valid() {
    return this.original.unchanged(this.candidate) && this.pending.matches(this.candidate)
  }

  commit() {
    this.original.attributes.restore(this.candidate)
    this.original.node.disabled = true
    this.pending = { original: new NativeUiStyleAttributes(this.original.node), candidate: new NativeUiStyleAttributes(this.candidate) }
  }

  retained() {
    return this.original.boundary(this.candidate) && this.pending.original.matches(this.original.node)
      && this.pending.candidate.matches(this.candidate)
  }

  rollback() {
    this.cancelLoad?.()
    this.original.attributes.restore(this.original.node)
    this.candidate.remove()
  }

  finalize() {
    const identifier = this.original.node.getAttribute('id')
    this.original.node.remove()
    this._identifier(identifier)
  }

  _identifier(identifier) {
    if (identifier !== null) this.candidate.setAttribute('id', identifier)
  }

  _load(deadline) {
    return new Promise((resolve, reject) => {
      let settled = false
      const finish = error => {
        if (settled) return
        settled = true
        this.cancelLoad = null
        clearTimeout(timer)
        this.candidate.removeEventListener('load', success)
        this.candidate.removeEventListener('error', failure)
        const completion = [resolve, reject][Number(error !== null)]
        completion(error)
      }
      const success = () => finish(null)
      const failure = () => finish(new Error('Stylesheet load failed'))
      const timer = setTimeout(() => finish(new Error('Stylesheet load deadline exceeded')), deadline)
      this.cancelLoad = () => finish(new Error('Stylesheet load cancelled'))
      this.candidate.addEventListener('load', success, { once: true })
      this.candidate.addEventListener('error', failure, { once: true })
    })
  }
}
