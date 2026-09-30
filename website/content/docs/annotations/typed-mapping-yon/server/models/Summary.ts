import type { ShortDetail } from '/models/ShortDetail.ts'
export interface Summary { id: number; name: string; detail: ShortDetail; items: ShortDetail[]; kind: string }
