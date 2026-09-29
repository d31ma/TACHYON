export class SocketProbe {
  static waitMilliseconds = 10_000
  static normalClosure = 1000

  /**
   * @param {string | URL} url
   * @param {typeof WebSocket} [Socket]
   * @returns {Promise<string>}
   */
  static firstMessage(url, Socket = WebSocket) {
    return new Promise((resolve, reject) => {
      const socket = new Socket(url)
      const state = { opened: false }
      const timer = setTimeout(() => {
        reject(new Error(SocketProbe.silentProblem(state.opened)))
        socket.close()
      }, SocketProbe.waitMilliseconds)
      socket.onopen = () => { state.opened = true }
      socket.onmessage = ({ data }) => {
        clearTimeout(timer)
        resolve(String(data))
        socket.close(SocketProbe.normalClosure)
      }
      socket.onclose = ({ code, reason }) => {
        clearTimeout(timer)
        reject(new Error(SocketProbe.closedProblem(state.opened, code, reason)))
      }
    })
  }

  /**
   * @param {boolean} opened
   * @returns {string}
   */
  static silentProblem(opened) {
    if (opened) return `The socket opened but sent no message within ${SocketProbe.waitMilliseconds} ms`
    return `The socket did not open within ${SocketProbe.waitMilliseconds} ms`
  }

  /**
   * @param {boolean} opened
   * @param {number} code
   * @param {string} reason
   * @returns {string}
   */
  static closedProblem(opened, code, reason) {
    const stage = opened ? 'after it opened' : 'before it opened'
    const because = reason ? `: ${reason}` : ' with no reason'
    return `The socket closed with ${code} ${stage} and before its first message${because}`
  }
}
