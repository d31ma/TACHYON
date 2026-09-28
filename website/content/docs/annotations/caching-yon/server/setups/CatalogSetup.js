@Setup
class CatalogSetup {
  static configure() {
    const entries = new Map();
    return {
      authorize: () => true,
      caches: {
        catalog: {
          get: (key) => entries.get(key) ?? { hit: false },
          set: (key, value, execution) => {
            entries.set(key, { hit: true, value, expiresAt: execution.expiresAt });
          },
          delete: (key) => {
            entries.delete(key);
          },
        },
      },
    };
  }
}
