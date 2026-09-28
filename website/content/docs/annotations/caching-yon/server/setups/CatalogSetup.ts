@Setup
class CatalogSetup {
  static configure() {
    const entries = new Map<string, unknown>();
    return {
      authorize: () => true,
      caches: {
        catalog: {
          get: (key: string) => entries.get(key) ?? { hit: false },
          set: (key: string, value: unknown, execution: { expiresAt: number }) => {
            entries.set(key, { hit: true, value, expiresAt: execution.expiresAt });
          },
          delete: (key: string) => {
            entries.delete(key);
          },
        },
      },
    };
  }
}
