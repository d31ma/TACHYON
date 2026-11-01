import { resolve } from 'node:path'

export class SiteArtifacts {
    static web(project = process.cwd()) {
        return resolve(project, 'dist', 'web')
    }
}
