import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

class TestRoots {
  static hooked = fileURLToPath(process.env.TAC_UNIT_HOOKED_URL)
  static client = fileURLToPath(process.env.TAC_UNIT_CLIENT_URL)
  static project = fileURLToPath(process.env.TAC_UNIT_PROJECT_URL)

  static redirected(specifier) {
    if (!/^\/(?!\/)/.test(specifier) || existsSync(specifier)) return undefined
    return [TestRoots.hooked, TestRoots.client].map((root) => join(root, specifier)).find((path) => existsSync(path))
  }
}

if (typeof Deno !== 'undefined') {
} else if (typeof Bun !== 'undefined') {
  Bun.plugin({
    name: 'tachyon-client-paths',
    setup(build) {
      build.onResolve({ filter: /^\/(?!\/)/ }, (args) => {
        const path = TestRoots.redirected(args.path)
        return path ? { path } : undefined
      })
    },
  })
} else {
  const { register } = await import('node:module')
  const hooks = `
    import { existsSync } from 'node:fs'
    import { join } from 'node:path'
    import { pathToFileURL } from 'node:url'
    class TestRoots {
      static all = ${JSON.stringify([TestRoots.hooked, TestRoots.client])}
    }
    export async function resolve(specifier, context, next) {
      const path = /^\\/(?!\\/)/.test(specifier) && !existsSync(specifier)
        ? TestRoots.all.map((root) => join(root, specifier)).find((candidate) => existsSync(candidate))
        : undefined
      return next(path ? pathToFileURL(path).href : specifier, context)
    }`
  register(`data:text/javascript,${encodeURIComponent(hooks)}`, pathToFileURL(join(TestRoots.project, 'noop.js')))
}

try {
  const registrator = createRequire(join(TestRoots.project, 'noop.js')).resolve('@happy-dom/global-registrator')
  const { GlobalRegistrator } = await import(pathToFileURL(registrator).href)
  GlobalRegistrator.register({ url: 'http://localhost/' })
} catch {
}
