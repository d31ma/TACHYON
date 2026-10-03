import type { FeatureEntry } from '/models/FeatureEntry.ts'

export interface FeatureCatalogue {
  groups: string[]
  features: FeatureEntry[]
}
