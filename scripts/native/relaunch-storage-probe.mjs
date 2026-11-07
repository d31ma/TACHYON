import { randomUUID } from 'node:crypto'

export class RelaunchStorageProbe {
  static WRITTEN = 'written'
  static KEPT = 'local=kept,store=kept,cache=kept'
  static CHECKING = 'checking'

  static pageModule() {
    const value = JSON.stringify(randomUUID())
    return `export class RelaunchStorage {
  static VALUE = ${value};
  static LOCAL_KEY = 'tachyon-relaunch:LocalState';
  static DATABASE = 'tachyon-relaunch';
  static TABLE = 'rows';
  static ROW_ID = 'row';
  static CACHE = 'tachyon-cache:relaunch';
  static CACHE_PATH = '/.tachyon/cache/relaunch/row';
  static CACHE_BASE = 'https://cache.tachyon.invalid';
  static CACHE_EXPIRES_HEADER = 'x-tachyon-expires-at';
  static CACHE_LIFETIME = 3600000;

  static async check() {
    try {
      return await RelaunchStorage.compare();
    } catch (error) {
      return 'error: ' + String(error);
    }
  }

  static async compare() {
    const database = await RelaunchStorage.openDatabase();
    try {
      if (localStorage.getItem(RelaunchStorage.LOCAL_KEY) !== RelaunchStorage.VALUE) {
        return await RelaunchStorage.writeAll(database);
      }
      return await RelaunchStorage.readAll(database);
    } finally {
      database.close();
    }
  }

  static async writeAll(database) {
    localStorage.setItem(RelaunchStorage.LOCAL_KEY, RelaunchStorage.VALUE);
    await RelaunchStorage.writeRow(database);
    if (!RelaunchStorage.cacheAvailable()) return '${RelaunchStorageProbe.WRITTEN},cache=unavailable';
    const cache = await caches.open(RelaunchStorage.CACHE);
    await cache.put(RelaunchStorage.cacheAddress(), RelaunchStorage.cacheResponse());
    return '${RelaunchStorageProbe.WRITTEN}';
  }

  static writeRow(database) {
    const writing = database.transaction(RelaunchStorage.TABLE, 'readwrite');
    writing.objectStore(RelaunchStorage.TABLE).put({ id: RelaunchStorage.ROW_ID, value: RelaunchStorage.VALUE });
    return new Promise((resolve, reject) => {
      writing.oncomplete = resolve;
      writing.onerror = () => reject(writing.error);
    });
  }

  static async readAll(database) {
    const local = RelaunchStorage.state(localStorage.getItem(RelaunchStorage.LOCAL_KEY));
    const reading = database.transaction(RelaunchStorage.TABLE).objectStore(RelaunchStorage.TABLE);
    const row = await RelaunchStorage.request(reading.get(RelaunchStorage.ROW_ID));
    const store = RelaunchStorage.state(row?.value);
    const cache = await RelaunchStorage.readCache();
    return ['local=' + local, 'store=' + store, 'cache=' + cache].join(',');
  }

  static async readCache() {
    if (!RelaunchStorage.cacheAvailable()) return 'unavailable';
    const cache = await caches.open(RelaunchStorage.CACHE);
    const response = await cache.match(RelaunchStorage.cacheAddress());
    if (!response) return 'missing';
    return RelaunchStorage.state(JSON.parse(await response.text()));
  }

  static cacheResponse() {
    const expiresAt = String(Date.now() + RelaunchStorage.CACHE_LIFETIME);
    const headers = { [RelaunchStorage.CACHE_EXPIRES_HEADER]: expiresAt };
    return new Response(JSON.stringify(RelaunchStorage.VALUE), { headers });
  }

  static state(value) {
    if (value === RelaunchStorage.VALUE) return 'kept';
    if (value === undefined || value === null) return 'missing';
    return 'changed';
  }

  static cacheAvailable() {
    return globalThis.isSecureContext === true && typeof globalThis.caches?.open === 'function';
  }

  static cacheAddress() {
    const webPage = ['http:', 'https:'].includes(location.protocol);
    const base = webPage ? location.origin : RelaunchStorage.CACHE_BASE;
    return new URL(RelaunchStorage.CACHE_PATH, base);
  }

  static openDatabase() {
    const opening = indexedDB.open(RelaunchStorage.DATABASE, 1);
    opening.onupgradeneeded = () => {
      opening.result.createObjectStore(RelaunchStorage.TABLE, { keyPath: 'id' });
    };
    return RelaunchStorage.request(opening);
  }

  static request(operation) {
    return new Promise((resolve, reject) => {
      operation.onsuccess = () => resolve(operation.result);
      operation.onerror = () => reject(operation.error);
    });
  }
}
`
  }
}
