import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export class CargoArtifacts {
    /** @param {...string} parts */
    static path(...parts) {
        const directory = process.env.CARGO_TARGET_DIR
            ? resolve(process.env.CARGO_TARGET_DIR)
            : fileURLToPath(new URL('../target/', import.meta.url))
        return resolve(directory, ...parts)
    }
}
