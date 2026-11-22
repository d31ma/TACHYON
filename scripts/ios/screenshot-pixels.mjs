import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { inflateSync } from 'node:zlib'

/**
 * @typedef {Object} ScreenshotImage
 * @property {number} width
 * @property {number} height
 * @property {(x: number, y: number) => number[]} pixel
 */

export class ScreenshotPixels {
  static SIGNATURE_BYTES = 8
  static CHUNK_EXTRA_BYTES = 12
  static COLOR_TYPES = { rgb: 2, rgba: 6 }
  static NEAR_DISTANCE = 8

  /** @param {string} file @returns {ScreenshotImage} */
  static read(file) {
    const { header, data } = ScreenshotPixels.chunks(readFileSync(file))
    ScreenshotPixels.checkFormat(header)
    const channels = header.color === ScreenshotPixels.COLOR_TYPES.rgba ? 4 : 3
    const stride = header.width * channels
    const pixels = ScreenshotPixels.unfilter(inflateSync(Buffer.concat(data)), header.height, stride, channels)
    return { width: header.width, height: header.height, pixel: (x, y) => ScreenshotPixels.at(pixels, stride, channels, x, y) }
  }

  /** @param {Buffer} bytes @returns {{ header: any, data: Buffer[] }} */
  static chunks(bytes) {
    let offset = ScreenshotPixels.SIGNATURE_BYTES
    let header
    const data = []
    while (offset < bytes.length) {
      const length = bytes.readUInt32BE(offset)
      const type = bytes.toString('latin1', offset + 4, offset + 8)
      const body = bytes.subarray(offset + 8, offset + 8 + length)
      if (type === 'IHDR') header = { width: body.readUInt32BE(0), height: body.readUInt32BE(4), depth: body[8], color: body[9], interlace: body[12] }
      if (type === 'IDAT') data.push(body)
      offset += length + ScreenshotPixels.CHUNK_EXTRA_BYTES
    }
    return { header, data }
  }

  /** @param {any} header */
  static checkFormat(header) {
    const known = Object.values(ScreenshotPixels.COLOR_TYPES).includes(header?.color)
    assert.ok(header && header.depth === 8 && known && header.interlace === 0, `unexpected screenshot format: ${JSON.stringify(header)}`)
  }

  /** @param {Buffer} raw @param {number} height @param {number} stride @param {number} channels @returns {Buffer} */
  static unfilter(raw, height, stride, channels) {
    const pixels = Buffer.alloc(stride * height)
    for (let row = 0; row < height; row++) {
      ScreenshotPixels.unfilterRow(raw, pixels, row, stride, channels)
    }
    return pixels
  }

  /** @param {Buffer} raw @param {Buffer} pixels @param {number} row @param {number} stride @param {number} channels */
  static unfilterRow(raw, pixels, row, stride, channels) {
    const filter = raw[row * (stride + 1)]
    const line = raw.subarray(row * (stride + 1) + 1, (row + 1) * (stride + 1))
    for (let index = 0; index < stride; index++) {
      const left = index >= channels ? pixels[row * stride + index - channels] : 0
      const up = row > 0 ? pixels[(row - 1) * stride + index] : 0
      const corner = row > 0 && index >= channels ? pixels[(row - 1) * stride + index - channels] : 0
      const before = [0, left, up, (left + up) >> 1, ScreenshotPixels.paeth(left, up, corner)][filter]
      assert.notEqual(before, undefined, `unknown PNG filter ${filter}`)
      pixels[row * stride + index] = (line[index] + before) & 0xff
    }
  }

  /** @param {number} left @param {number} up @param {number} corner @returns {number} */
  static paeth(left, up, corner) {
    const guess = left + up - corner
    const [toLeft, toUp, toCorner] = [left, up, corner].map((value) => Math.abs(guess - value))
    if (toLeft <= toUp && toLeft <= toCorner) return left
    return toUp <= toCorner ? up : corner
  }

  /** @param {Buffer} pixels @param {number} stride @param {number} channels @param {number} x @param {number} y @returns {number[]} */
  static at(pixels, stride, channels, x, y) {
    const start = Math.round(y) * stride + Math.round(x) * channels
    return [pixels[start], pixels[start + 1], pixels[start + 2]]
  }

  /** @param {string} hex @returns {number[]} */
  static rgb(hex) {
    return [1, 3, 5].map((start) => Number.parseInt(hex.slice(start, start + 2), 16))
  }

  /** @param {number[]} pixel @param {string} hex @returns {boolean} */
  static near(pixel, hex) {
    return ScreenshotPixels.rgb(hex).every((channel, index) => Math.abs(channel - pixel[index]) <= ScreenshotPixels.NEAR_DISTANCE)
  }

  /** @param {ScreenshotImage} image @param {number} y @param {string} hex @returns {number} */
  static rowShare(image, y, hex) {
    let matches = 0
    for (let x = 0; x < image.width; x++) if (ScreenshotPixels.near(image.pixel(x, y), hex)) matches++
    return matches / image.width
  }

  /** @param {ScreenshotImage} image @param {number} x @param {string} hex @returns {number} */
  static columnShare(image, x, hex) {
    let matches = 0
    for (let y = 0; y < image.height; y++) if (ScreenshotPixels.near(image.pixel(x, y), hex)) matches++
    return matches / image.height
  }
}
