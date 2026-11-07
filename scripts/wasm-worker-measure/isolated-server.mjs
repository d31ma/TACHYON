import { createReadStream, existsSync, readdirSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export class IsolatedServer {
  static probePrefix = '/__probe/';
  static probeFiles = path.dirname(fileURLToPath(import.meta.url));
  static indexFile = 'index.html';
  static dynamicPrefix = '_';
  static trailingSlash = /\/$/;
  static notFound = 404;
  static found = 200;
  static isolationHeaders = {
    'cross-origin-opener-policy': 'same-origin',
    'cross-origin-embedder-policy': 'credentialless',
    'cross-origin-resource-policy': 'same-origin',
    'cache-control': 'no-store',
  };
  /** @type {Record<string, string>} */
  static contentTypes = {
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.json': 'application/json',
    '.webmanifest': 'application/manifest+json',
    '.wasm': 'application/wasm',
    '.svg': 'image/svg+xml',
    '.png': 'image/png',
    '.woff2': 'font/woff2',
    '.dat': 'application/octet-stream',
    '.txt': 'text/plain; charset=utf-8',
  };

  /**
   * @param {string} root
   * @returns {Promise<{origin: string, close: () => Promise<void>}>}
   */
  static async start(root) {
    const server = createServer((request, response) => IsolatedServer._serve(root, request, response));
    await IsolatedServer._listen(server);
    const address = /** @type {import('node:net').AddressInfo} */ (server.address());
    return { origin: `http://127.0.0.1:${address.port}`, close: () => IsolatedServer._close(server) };
  }

  /**
   * @param {string} root
   * @param {import('node:http').IncomingMessage} request
   * @param {import('node:http').ServerResponse} response
   * @returns {void}
   */
  static _serve(root, request, response) {
    const file = IsolatedServer._fileFor(root, new URL(/** @type {string} */ (request.url), 'http://localhost').pathname);
    if (!file) {
      response.writeHead(IsolatedServer.notFound, IsolatedServer.isolationHeaders).end();
      return;
    }
    response.writeHead(IsolatedServer.found, { ...IsolatedServer.isolationHeaders, 'content-type': IsolatedServer._typeOf(file) });
    createReadStream(file).pipe(response);
  }

  /**
   * @param {string} root
   * @param {string} pathname
   * @returns {string | null}
   */
  static _fileFor(root, pathname) {
    const base = pathname.startsWith(IsolatedServer.probePrefix) ? IsolatedServer.probeFiles : root;
    const relative = decodeURIComponent(pathname.replace(IsolatedServer.probePrefix, '/'));
    const requested = path.join(base, path.normalize(relative));
    return IsolatedServer._existing(base, IsolatedServer._withIndex(requested)) ?? IsolatedServer._existing(base, IsolatedServer._dynamicRoute(requested));
  }

  /**
   * @param {string} requested
   * @returns {string}
   */
  static _dynamicRoute(requested) {
    const route = requested.replace(IsolatedServer.trailingSlash, '');
    const parent = path.dirname(route);
    const dynamic = IsolatedServer._folders(parent).find((name) => name.startsWith(IsolatedServer.dynamicPrefix));
    return path.join(parent, dynamic ?? path.basename(route), IsolatedServer.indexFile);
  }

  /**
   * @param {string} folder
   * @returns {string[]}
   */
  static _folders(folder) {
    if (!existsSync(folder)) return [];
    return readdirSync(folder, { withFileTypes: true }).filter((entry) => entry.isDirectory()).map((entry) => entry.name);
  }

  /**
   * @param {string} file
   * @returns {string}
   */
  static _withIndex(file) {
    if (existsSync(file) && statSync(file).isDirectory()) return path.join(file, IsolatedServer.indexFile);
    return file;
  }

  /**
   * @param {string} base
   * @param {string} file
   * @returns {string | null}
   */
  static _existing(base, file) {
    if (!file.startsWith(base) || !existsSync(file)) return null;
    return file;
  }

  /**
   * @param {string} file
   * @returns {string}
   */
  static _typeOf(file) {
    return IsolatedServer.contentTypes[path.extname(file)] ?? 'application/octet-stream';
  }

  /**
   * @param {import('node:http').Server} server
   * @returns {Promise<void>}
   */
  static _listen(server) {
    return new Promise((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '127.0.0.1', resolve);
    });
  }

  /**
   * @param {import('node:http').Server} server
   * @returns {Promise<void>}
   */
  static _close(server) {
    /** @type {Promise<void>} */
    const closed = new Promise((resolve) => server.close(() => resolve(undefined)));
    server.closeAllConnections();
    return closed;
  }
}
