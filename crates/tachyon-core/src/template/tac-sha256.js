class TacSha256 {
  static rounds = Uint32Array.from([
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ])
  static first = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19]
  static blockBytes = 64
  static lengthAt = 56
  static wordShift = 0x100000000

  state = Uint32Array.from(TacSha256.first)
  block = new Uint8Array(TacSha256.blockBytes)
  words = new Uint32Array(TacSha256.rounds.length)
  filled = 0
  length = 0

  static rotate(word, count) {
    return (word >>> count) | (word << (32 - count))
  }

  update(bytes) {
    this.length += bytes.length
    let index = 0
    while (index < bytes.length) {
      const taken = Math.min(TacSha256.blockBytes - this.filled, bytes.length - index)
      this.block.set(bytes.subarray(index, index + taken), this.filled)
      this.filled += taken
      index += taken
      this.mixWhenFull()
    }
    return this
  }

  mixWhenFull() {
    if (this.filled === TacSha256.blockBytes) this.mix()
  }

  mix() {
    this.spread()
    const next = [...this.state]
    for (let round = 0; round < TacSha256.rounds.length; round += 1) this.round(next, round)
    next.forEach((word, index) => { this.state[index] += word })
    this.filled = 0
  }

  spread() {
    const view = new DataView(this.block.buffer)
    const words = this.words
    for (let index = 0; index < words.length; index += 1) words[index] = index < 16 ? view.getUint32(index * 4) : this.spreadWord(index)
  }

  spreadWord(index) {
    const { rotate } = TacSha256
    const early = this.words[index - 15]
    const late = this.words[index - 2]
    const low = rotate(early, 7) ^ rotate(early, 18) ^ (early >>> 3)
    const high = rotate(late, 17) ^ rotate(late, 19) ^ (late >>> 10)
    return (this.words[index - 16] + low + this.words[index - 7] + high) | 0
  }

  round(next, round) {
    const { rotate } = TacSha256
    const [a, b, c, d, e, f, g, h] = next
    const chosen = (e & f) ^ (~e & g)
    const major = (a & b) ^ (a & c) ^ (b & c)
    const first = (h + (rotate(e, 6) ^ rotate(e, 11) ^ rotate(e, 25)) + chosen + TacSha256.rounds[round] + this.words[round]) | 0
    const second = ((rotate(a, 2) ^ rotate(a, 13) ^ rotate(a, 22)) + major) | 0
    next.splice(0, 8, (first + second) | 0, a, b, c, (d + first) | 0, e, f, g)
  }

  digest() {
    const bits = this.length * 8
    const padding = new Uint8Array((this.filled < TacSha256.lengthAt ? TacSha256.lengthAt : TacSha256.lengthAt + TacSha256.blockBytes) - this.filled)
    padding[0] = 0x80
    this.update(padding)
    const tail = new DataView(new ArrayBuffer(8))
    tail.setUint32(0, Math.floor(bits / TacSha256.wordShift))
    tail.setUint32(4, bits >>> 0)
    this.update(new Uint8Array(tail.buffer))
    return [...this.state].map((word) => word.toString(16).padStart(8, '0')).join('')
  }
}
