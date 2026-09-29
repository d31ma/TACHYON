@Setup
export default class CatalogSetup {
  /** @returns {BehaviorConfiguration} */
  static configure() {
    /** @type {Map<string, BehaviorCacheResult>} */
    const entries = new Map();
    return {
      authorize: () => true,
      caches: {
        catalog: {
          get: (/** @type {string} */ key) => entries.get(key) ?? { hit: false },
          set: (/** @type {string} */ key, /** @type {BehaviorJsonValue} */ value, /** @type {BehaviorCacheWriteExecution} */ execution) => {
            entries.set(key, { hit: true, value, expiresAt: execution.expiresAt });
          },
          delete: (/** @type {string} */ key) => {
            entries.delete(key);
          },
        },
      },
    };
  }
}
