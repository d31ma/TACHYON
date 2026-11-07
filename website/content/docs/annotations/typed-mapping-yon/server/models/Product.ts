import type { Detail } from '/models/Detail.ts'
export interface Product { id: number; name: string; detail: Detail; items: Detail[]; secret: string }
