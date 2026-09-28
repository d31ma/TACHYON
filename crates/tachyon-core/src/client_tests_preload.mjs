import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

class TestRoots {
  static client = process.env.TAC_UNIT_CLIENT_ROOT
  static project = process.env.TAC_UNIT_PROJECT_ROOT
}

const redirected = (specifier) => {
  if (!/^\/(?!\/)/.test(specifier) || existsSync(specifier)) return undefined
  const clientPath = TestRoots.client + specifier
  return existsSync(clientPath) ? clientPath : undefined
}

if (typeof Deno !== 'undefined') {
} else if (typeof Bun !== 'undefined') {
  Bun.plugin({
    name: 'tachyon-client-paths',
    setup(build) {
      build.onResolve({ filter: /^\/(?!\/)/ }, (args) => {
        const path = redirected(args.path)
        return path ? { path } : undefined
      })
    },
  })
} else {
  const { register } = await import('node:module')
  const hooks = `
    import { existsSync } from 'node:fs'
    import { pathToFileURL } from 'node:url'
    class TestRoots {
      static client = ${JSON.stringify(TestRoots.client)}
    }
    export async function resolve(specifier, context, next) {
      if (/^\\/(?!\\/)/.test(specifier) && !existsSync(specifier) && existsSync(TestRoots.client + specifier)) {
        return next(pathToFileURL(TestRoots.client + specifier).href, context)
      }
      return next(specifier, context)
    }`
  register(`data:text/javascript,${encodeURIComponent(hooks)}`, pathToFileURL(join(TestRoots.project, 'noop.js')))
}

try {
  const registrator = createRequire(join(TestRoots.project, 'noop.js')).resolve('@happy-dom/global-registrator')
  const { GlobalRegistrator } = await import(pathToFileURL(registrator).href)
  GlobalRegistrator.register({ url: 'http://localhost/' })
} catch {
}
