type VersionedPayload = {
    message: string
    version: string
}

export default class {
    versioned: VersionedPayload | null = null

    async mount(): Promise<void> {
        try {
            const response = await fetch('/language/versions/v1', { cache: 'reload' })
            this.versioned = await response.json() as VersionedPayload
        } catch {
            this.versioned = null
        }
    }
}
