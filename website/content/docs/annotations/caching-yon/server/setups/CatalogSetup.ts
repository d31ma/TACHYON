@Setup
export default class CatalogSetup {
  static configure(): BehaviorConfiguration {
    const entries = new Map<string, BehaviorCacheResult>();
    return {
      authorize: () => true,
      caches: {
        catalog: {
          get: (key: string) => entries.get(key) ?? { hit: false },
          set: (key: string, value: BehaviorJsonValue, execution: BehaviorCacheWriteExecution) => {
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
