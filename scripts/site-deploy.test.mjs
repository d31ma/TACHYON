import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { describe, mock, test } from 'node:test'
import { SiteDeploy } from './site-deploy.mjs'

const CONFIG = {
    appId: 'd000000000000',
    region: 'ca-central-1',
    domain: 'site.example',
    roles: {},
    requiredHeaders: { 'x-content-type-options': 'nosniff' },
    probes: [{ path: '/', contains: 'Welcome', contentTypes: ['text/html'] }]
}

function fakeTools(responses = {}) {
    const calls = []
    const fetched = []
    const logs = []
    const jobStates = [...(responses.jobStates ?? [])]
    const pages = responses.pages ?? {}
    return {
        calls,
        fetched,
        logs,
        tools: {
            run: (command, args, options) => {
                calls.push([command, ...args])
                if (command === 'zip') return SiteDeploy.run(command, args, options)
                if (args[1] === 'create-deployment') return JSON.stringify({ jobId: '7', zipUploadUrl: 'https://upload.example/zip' })
                if (args[1] === 'get-job') return JSON.stringify({ job: { summary: { status: jobStates.shift() } } })
                if (args[1] === 'get-app') return JSON.stringify({ app: { customHeaders: responses.policy ?? '' } })
                return ''
            },
            fetch: async (url, options = {}) => {
                fetched.push([url, options.method ?? 'GET'])
                const page = pages[url.split('?')[0]] ?? { status: 404, body: '' }
                return new Response(page.body, { status: page.status ?? 200, headers: page.headers ?? {} })
            },
            sleep: async () => {},
            log: (line) => logs.push(line)
        }
    }
}

function temporary(prefix) {
    return mkdtempSync(join(tmpdir(), prefix))
}

function siteDirectory() {
    const directory = temporary('site-deploy-')
    mkdirSync(join(directory, 'docs'))
    writeFileSync(join(directory, 'index.html'), '<h1>Welcome</h1>')
    writeFileSync(join(directory, 'docs', 'index.html'), '<h1>Docs</h1>')
    return directory
}

function zipFile(content = 'zip') {
    const file = join(temporary('site-up-'), 'site.zip')
    writeFileSync(file, content)
    return file
}

function configFile() {
    const path = join(temporary('site-config-'), 'site-deploy.json')
    writeFileSync(path, JSON.stringify(CONFIG))
    return path
}

function stagingSite() {
    return {
        'https://upload.example/zip': { status: 200, body: '' },
        'https://staging.site.example/site-version.json': { body: '{"version":"26.46.01"}' },
        'https://staging.site.example/': { body: 'Welcome', headers: { 'content-type': 'text/html', 'x-content-type-options': 'nosniff' } }
    }
}

async function printed(argv, fake) {
    const log = mock.method(console, 'log', () => {})
    await SiteDeploy.main(argv, new SiteDeploy(fake.tools))
    const output = log.mock.calls.at(-1).arguments[0]
    log.mock.restore()
    return output
}

describe('SiteDeploy.package', () => {
    test('builds the same zip twice from the same site', () => {
        const site = new SiteDeploy(fakeTools().tools)
        const first = site.package(siteDirectory(), '26.46.01', 'abc123', join(temporary('site-zip-'), 'a.zip'))
        const second = site.package(siteDirectory(), '26.46.01', 'abc123', join(temporary('site-zip-'), 'b.zip'))
        assert.equal(first, second)
        assert.match(first, /^[0-9a-f]{64}$/)
    })

    test('writes site-version.json into the site', () => {
        const directory = siteDirectory()
        new SiteDeploy(fakeTools().tools).package(directory, '26.46.01', 'abc123', join(temporary('site-zip-'), 'site.zip'))
        assert.deepEqual(JSON.parse(readFileSync(join(directory, 'site-version.json'), 'utf8')), { version: '26.46.01', commit: 'abc123' })
    })

    test('stores the files in sorted order with no directory entries', () => {
        const zip = join(temporary('site-zip-'), 'site.zip')
        new SiteDeploy(fakeTools().tools).package(siteDirectory(), '26.46.01', 'abc123', zip)
        assert.deepEqual(SiteDeploy.run('unzip', ['-Z1', zip]).trim().split('\n'), ['docs/index.html', 'index.html', 'site-version.json'])
    })
})

describe('SiteDeploy.verify', () => {
    test('accepts the matching checksum', () => {
        const file = zipFile()
        const site = new SiteDeploy(fakeTools().tools)
        assert.doesNotThrow(() => site.verify(file, `${site.checksum(file)}\n`))
    })

    test('rejects a different checksum', () => {
        assert.throws(() => new SiteDeploy(fakeTools().tools).verify(zipFile(), '0'.repeat(64)), /SHA-256/)
    })
})

describe('SiteDeploy.checkHeaders', () => {
    test('accepts a policy whose folded value contains the required header', () => {
        const policy = "customHeaders:\n  - pattern: '**/*'\n    headers:\n      - key: X-Content-Type-Options\n        value: >-\n          nosniff\n"
        assert.doesNotThrow(() => new SiteDeploy(fakeTools({ policy }).tools).checkHeaders(CONFIG))
    })

    test('names the headers the live policy lacks', () => {
        assert.throws(() => new SiteDeploy(fakeTools({ policy: '' }).tools).checkHeaders(CONFIG), /x-content-type-options/)
    })

    test('reads the policy with GetApp only', () => {
        const fake = fakeTools({ policy: 'x-content-type-options nosniff' })
        new SiteDeploy(fake.tools).checkHeaders(CONFIG)
        assert.deepEqual(fake.calls, [['aws', 'amplify', 'get-app', '--app-id', CONFIG.appId, '--region', 'ca-central-1', '--output', 'json']])
    })
})

describe('SiteDeploy.deploy', () => {
    test('uploads the zip, starts the job and waits until it succeeds', async () => {
        const fake = fakeTools({ jobStates: ['RUNNING', 'SUCCEED'], pages: stagingSite() })
        await new SiteDeploy(fake.tools).deploy(CONFIG, 'staging', zipFile())
        assert.deepEqual(fake.calls.map((call) => call[2]), ['create-deployment', 'start-deployment', 'get-job', 'get-job'])
        assert.ok(fake.calls[1].includes('7'))
        assert.deepEqual(fake.fetched, [['https://upload.example/zip', 'PUT']])
    })

    test('fails when the Amplify job fails', async () => {
        const fake = fakeTools({ jobStates: ['FAILED'], pages: stagingSite() })
        await assert.rejects(new SiteDeploy(fake.tools).deploy(CONFIG, 'production', zipFile()), /FAILED/)
    })

    test('fails when the Amplify job never finishes', async () => {
        const fake = fakeTools({ jobStates: Array(SiteDeploy.JOB_ATTEMPTS).fill('RUNNING'), pages: stagingSite() })
        await assert.rejects(new SiteDeploy(fake.tools).deploy(CONFIG, 'staging', zipFile()), /did not finish/)
    })

    test('fails before starting when the upload is refused', async () => {
        const fake = fakeTools({ pages: { 'https://upload.example/zip': { status: 403, body: '' } } })
        await assert.rejects(new SiteDeploy(fake.tools).deploy(CONFIG, 'staging', zipFile()), /HTTP 403/)
        assert.deepEqual(fake.calls.map((call) => call[2]), ['create-deployment'])
    })
})

describe('SiteDeploy.liveVersion', () => {
    test('reads the version the branch serves', async () => {
        assert.equal(await new SiteDeploy(fakeTools({ pages: stagingSite() }).tools).liveVersion(CONFIG, 'staging'), '26.46.01')
    })

    test('is empty when the site has no version file yet', async () => {
        assert.equal(await new SiteDeploy(fakeTools().tools).liveVersion(CONFIG, 'production'), '')
    })
})

describe('SiteDeploy.probe', () => {
    test('passes when the version, content, type and headers match', async () => {
        const fake = fakeTools({
            pages: {
                'https://site.example/site-version.json': { body: '{"version":"26.46.01"}' },
                'https://site.example/': { body: 'Welcome', headers: { 'content-type': 'text/html; charset=utf-8', 'x-content-type-options': 'nosniff' } }
            }
        })
        await new SiteDeploy(fake.tools).probe(CONFIG, 'production', '26.46.01')
        assert.equal(fake.logs.at(-1), 'https://site.example serves 26.46.01; 1 probes passed')
    })

    test('lists every failed check', async () => {
        const fake = fakeTools({
            pages: {
                'https://site.example/site-version.json': { body: '{"version":"26.46.01"}' },
                'https://site.example/': { body: 'Other', headers: { 'content-type': 'text/plain' } }
            }
        })
        await assert.rejects(
            new SiteDeploy(fake.tools).probe(CONFIG, 'production', '26.46.01'),
            { message: 'Probes failed: / lacks "Welcome"; / has content type text/plain; / lacks header x-content-type-options' }
        )
    })

    test('accepts any content type when the probe lists none', async () => {
        const fake = fakeTools({
            pages: {
                'https://site.example/site-version.json': { body: '{"version":"26.46.01"}' },
                'https://site.example/': { body: 'Welcome', headers: { 'content-type': 'text/plain', 'x-content-type-options': 'nosniff' } }
            }
        })
        const config = { ...CONFIG, probes: [{ path: '/', contains: 'Welcome', contentTypes: [] }] }
        await new SiteDeploy(fake.tools).probe(config, 'production', '26.46.01')
        assert.equal(fake.logs.length, 1)
    })

    test('fails when the branch never serves the new version', async () => {
        const fake = fakeTools({ pages: { 'https://staging.site.example/site-version.json': { body: '{"version":"26.45.07"}' } } })
        await assert.rejects(new SiteDeploy(fake.tools).probe(CONFIG, 'staging', '26.46.01'), /does not serve 26.46.01/)
    })
})

describe('SiteDeploy.origin', () => {
    test('serves production from the domain itself', () => {
        assert.equal(new SiteDeploy(fakeTools().tools).origin(CONFIG, 'production'), 'https://site.example')
    })

    test('serves staging from its subdomain', () => {
        assert.equal(new SiteDeploy(fakeTools().tools).origin(CONFIG, 'staging'), 'https://staging.site.example')
    })
})

describe('SiteDeploy.main', () => {
    test('package prints the zip checksum', async () => {
        assert.match(await printed(['package', siteDirectory(), '26.46.01', 'abc123', join(temporary('site-zip-'), 's.zip')], fakeTools()), /^[0-9a-f]{64}$/)
    })

    test('verify confirms a matching checksum', async () => {
        const file = zipFile()
        assert.equal(await printed(['verify', file, new SiteDeploy(fakeTools().tools).checksum(file)], fakeTools()), 'verified')
    })

    test('check-headers reads the config file', async () => {
        assert.equal(await printed(['check-headers', configFile()], fakeTools({ policy: 'x-content-type-options: nosniff' })), 'header policy covers the required headers')
    })

    test('deploy runs the Amplify deployment', async () => {
        assert.equal(await printed(['deploy', configFile(), 'staging', zipFile()], fakeTools({ jobStates: ['SUCCEED'], pages: stagingSite() })), 'deployed')
    })

    test('live-version prints the served version', async () => {
        assert.equal(await printed(['live-version', configFile(), 'staging'], fakeTools({ pages: stagingSite() })), '26.46.01')
    })

    test('probe reports the probes passed', async () => {
        assert.equal(await printed(['probe', configFile(), 'staging', '26.46.01'], fakeTools({ pages: stagingSite() })), 'probes passed')
    })

    test('an unknown command prints the usage', async () => {
        await assert.rejects(SiteDeploy.main(['publish'], new SiteDeploy(fakeTools().tools)), /Usage/)
    })
})

describe('SiteDeploy.run', () => {
    test('returns the standard output of a command', () => {
        assert.equal(SiteDeploy.run('printf', ['ready']), 'ready')
    })

    test('fails when the command fails', () => {
        assert.throws(() => SiteDeploy.run('false', []), /false failed/)
    })

    test('live tools are wired to the real process', () => {
        assert.equal(SiteDeploy.live().tools.run, SiteDeploy.run)
    })
})
