export class FakeSocket {
  /** @type {{code:number, reason:string} | null} */
  static serverClose = null

  /** @param {string} _url */
  constructor(_url) {
    /** @type {() => void} */
    this.onopen = () => {}
    /** @type {(event: {data: string}) => void} */
    this.onmessage = () => {}
    /** @type {(event: {code: number, reason: string}) => void} */
    this.onclose = () => {}
    queueMicrotask(() => this.talk())
  }

  talk() {
    this.onopen()
    if (FakeSocket.serverClose) this.onclose(FakeSocket.serverClose)
    else this.onmessage({ data: '{"text":"Welcome to the room"}' })
  }

  /** @param {number} [code] */
  close(code = 1005) {
    this.onclose({ code, reason: '' })
  }
}
