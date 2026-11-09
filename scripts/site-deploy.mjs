import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { chmodSync, readFileSync, readdirSync, utimesSync, writeFileSync } from 'node:fs'
import { join, relative } from 'node:path'

/**
 * @typedef {{ path: string, contains: string, contentTypes: string[] }} Probe
 * @typedef {{ appId: string, region: string, domain: string, roles: Record<string, string>, requiredHeaders: Record<string, string>, probes: Probe[] }} SiteConfig
 * @typedef {{ cwd?: string, input?: string, env?: Record<string, string> }} RunOptions
 * @typedef {(command: string, args: string[], options?: RunOptions) => string} Runner
 * @typedef {{ run: Runner, fetch: typeof fetch, sleep: (seconds: number) => Promise<void>, log: (line: string) => void, mask: (secret: string) => void }} Tools
 * @typedef {{ jobId: string, zipUploadUrl: string }} Deployment
 */
export class SiteDeploy {
    static VERSION_FILE = 'site-version.json'
    static FIXED_TIME = new Date('1980-01-01T00:00:00Z')
    static FILE_MODE = 0o644
    static DIRECTORY_MODE = 0o755
    static SUCCEEDED = 'SUCCEED'
    static FAILED_STATES = ['FAILED', 'CANCELLED']
    static JOB_ATTEMPTS = 80
    static JOB_WAIT_SECONDS = 15
    static VERSION_ATTEMPTS = 12
    static VERSION_WAIT_SECONDS = 15
    static PRODUCTION = 'production'
    static AMPLIFY_DOMAIN = 'amplifyapp.com'

    /** @param {Tools} tools */
    constructor(tools) {
        this.tools = tools
    }

    /**
     * @param {string} directory
     * @param {string} version
     * @param {string} commit
     * @param {string} output
     * @returns {string}
     */
    package(directory, version, commit, output) {
        writeFileSync(join(directory, SiteDeploy.VERSION_FILE), `${JSON.stringify({ version, commit })}\n`)
        const files = this.#normalizeTree(directory)
        this.tools.run('zip', ['-X', '-D', '-q', output, '-@'], { cwd: directory, input: `${files.join('\n')}\n`, env: { TZ: 'UTC' } })
        return this.checksum(output)
    }

    /**
     * @param {string} file
     * @returns {string}
     */
    checksum(file) {
        return createHash('sha256').update(readFileSync(file)).digest('hex')
    }

    /**
     * @param {string} file
     * @param {string} expected
     */
    verify(file, expected) {
        this.#requireEqual(this.checksum(file), expected.trim(), `SHA-256 of ${file}`)
    }

    /** @param {SiteConfig} config */
    checkHeaders(config) {
        const policy = this.#headerPolicy(config)
        const missing = this.#missingFromPolicy(policy, config.requiredHeaders)
        this.#requireNothingMissing(missing, 'The Amplify header policy lacks')
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @param {string} zip
     */
    async deploy(config, branch, zip) {
        const deployment = /** @type {Deployment} */ (this.#aws('create-deployment', ['--app-id', config.appId, '--branch-name', branch], config.region))
        await this.#upload(deployment.zipUploadUrl, zip)
        this.#aws('start-deployment', ['--app-id', config.appId, '--branch-name', branch, '--job-id', deployment.jobId], config.region)
        await this.#waitForJob(config, branch, deployment.jobId)
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @returns {Promise<string>}
     */
    async liveVersion(config, branch) {
        return this.#fetchVersion(config, branch, this.#requestHeaders(config, branch))
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @param {string} version
     */
    async probe(config, branch, version) {
        const headers = this.#requestHeaders(config, branch)
        await this.#waitForVersion(config, branch, version, headers)
        const failures = await this.#probeFailures(config, this.origin(config, branch), headers)
        this.#requireNothingMissing(failures, 'Probes failed')
        this.tools.log(`${this.origin(config, branch)} serves ${version}; ${config.probes.length} probes passed`)
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @returns {string}
     */
    origin(config, branch) {
        return `https://${this.#host(config, branch)}`
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @returns {string}
     */
    #host(config, branch) {
        return branch === SiteDeploy.PRODUCTION ? config.domain : `${branch}.${config.appId}.${SiteDeploy.AMPLIFY_DOMAIN}`
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @returns {Record<string, string>}
     */
    #requestHeaders(config, branch) {
        const settings = this.#aws('get-branch', ['--app-id', config.appId, '--branch-name', branch], config.region).branch
        return this.#authorization(settings)
    }

    /**
     * @param {{ enableBasicAuth: boolean, basicAuthCredentials?: string }} settings
     * @returns {Record<string, string>}
     */
    #authorization(settings) {
        if (!settings.enableBasicAuth) return {}
        return { authorization: `Basic ${this.#maskedCredentials(settings.basicAuthCredentials)}` }
    }

    /**
     * @param {string | undefined} credentials
     * @returns {string}
     */
    #maskedCredentials(credentials) {
        if (!credentials) throw new Error('Basic auth is on for the branch, but GetBranch returned no basicAuthCredentials')
        this.tools.mask(credentials)
        return credentials
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @param {Record<string, string>} headers
     * @returns {Promise<string>}
     */
    async #fetchVersion(config, branch, headers) {
        const response = await this.tools.fetch(this.#versionUrl(config, branch), { redirect: 'follow', headers })
        return this.#versionFrom(response)
    }

    /**
     * @param {SiteConfig} config
     * @returns {string}
     */
    #headerPolicy(config) {
        return this.#aws('get-app', ['--app-id', config.appId], config.region).app.customHeaders ?? ''
    }

    /**
     * @param {string} directory
     * @returns {string[]}
     */
    #normalizeTree(directory) {
        const files = []
        for (const entry of readdirSync(directory, { recursive: true, withFileTypes: true })) {
            files.push(...this.#normalizeEntry(directory, join(entry.parentPath, entry.name), entry.isDirectory()))
        }
        utimesSync(directory, SiteDeploy.FIXED_TIME, SiteDeploy.FIXED_TIME)
        return files.sort()
    }

    /**
     * @param {string} directory
     * @param {string} path
     * @param {boolean} isDirectory
     * @returns {string[]}
     */
    #normalizeEntry(directory, path, isDirectory) {
        chmodSync(path, this.#mode(isDirectory))
        utimesSync(path, SiteDeploy.FIXED_TIME, SiteDeploy.FIXED_TIME)
        return this.#zipEntries(directory, path, isDirectory)
    }

    /**
     * @param {boolean} isDirectory
     * @returns {number}
     */
    #mode(isDirectory) {
        return isDirectory ? SiteDeploy.DIRECTORY_MODE : SiteDeploy.FILE_MODE
    }

    /**
     * @param {string} directory
     * @param {string} path
     * @param {boolean} isDirectory
     * @returns {string[]}
     */
    #zipEntries(directory, path, isDirectory) {
        return isDirectory ? [] : [relative(directory, path)]
    }

    /**
     * @param {string} operation
     * @param {string[]} args
     * @param {string} region
     * @returns {any}
     */
    #aws(operation, args, region) {
        return JSON.parse(this.tools.run('aws', ['amplify', operation, ...args, '--region', region, '--output', 'json']) || '{}')
    }

    /**
     * @param {string} url
     * @param {string} zip
     */
    async #upload(url, zip) {
        const response = await this.tools.fetch(url, { method: 'PUT', body: readFileSync(zip) })
        this.#requireOk(response, 'Upload of the site zip')
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @param {string} jobId
     */
    async #waitForJob(config, branch, jobId) {
        for (let attempt = 0; attempt < SiteDeploy.JOB_ATTEMPTS; attempt += 1) {
            const status = this.#aws('get-job', ['--app-id', config.appId, '--branch-name', branch, '--job-id', jobId], config.region).job.summary.status
            this.tools.log(`Amplify ${branch} job ${jobId}: ${status}`)
            if (this.#jobFinished(status)) return
            await this.tools.sleep(SiteDeploy.JOB_WAIT_SECONDS)
        }
        throw new Error(`Amplify ${branch} job ${jobId} did not finish in time`)
    }

    /**
     * @param {string} status
     * @returns {boolean}
     */
    #jobFinished(status) {
        if (SiteDeploy.FAILED_STATES.includes(status)) throw new Error(`Amplify job ended ${status}`)
        return status === SiteDeploy.SUCCEEDED
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @param {string} version
     * @param {Record<string, string>} headers
     */
    async #waitForVersion(config, branch, version, headers) {
        for (let attempt = 0; attempt < SiteDeploy.VERSION_ATTEMPTS; attempt += 1) {
            if ((await this.#fetchVersion(config, branch, headers)) === version) return
            await this.tools.sleep(SiteDeploy.VERSION_WAIT_SECONDS)
        }
        throw new Error(`${this.origin(config, branch)} does not serve ${version}`)
    }

    /**
     * @param {SiteConfig} config
     * @param {string} branch
     * @returns {string}
     */
    #versionUrl(config, branch) {
        return `${this.origin(config, branch)}/${SiteDeploy.VERSION_FILE}?check=${Date.now()}`
    }

    /**
     * @param {Response} response
     * @returns {Promise<string>}
     */
    async #versionFrom(response) {
        return response.ok ? this.#versionField(await response.json()) : ''
    }

    /**
     * @param {{ version?: string }} record
     * @returns {string}
     */
    #versionField(record) {
        return record.version ?? ''
    }

    /**
     * @param {SiteConfig} config
     * @param {string} origin
     * @param {Record<string, string>} headers
     * @returns {Promise<string[]>}
     */
    async #probeFailures(config, origin, headers) {
        const failures = []
        for (const probe of config.probes) {
            failures.push(...(await this.#checkProbe(origin, probe, config.requiredHeaders, headers)))
        }
        return failures
    }

    /**
     * @param {string} origin
     * @param {Probe} probe
     * @param {Record<string, string>} requiredHeaders
     * @param {Record<string, string>} headers
     * @returns {Promise<string[]>}
     */
    async #checkProbe(origin, probe, requiredHeaders, headers) {
        const response = await this.tools.fetch(`${origin}${probe.path}`, { redirect: 'follow', headers })
        const body = await response.text()
        return [
            ...this.#statusFailure(probe, response),
            ...this.#contentFailure(probe, body),
            ...this.#typeFailure(probe, this.#contentType(response)),
            ...this.#missingFromResponse(probe, response.headers, requiredHeaders),
        ]
    }

    /**
     * @param {Probe} probe
     * @param {Response} response
     * @returns {string[]}
     */
    #statusFailure(probe, response) {
        return response.ok ? [] : [`${probe.path} returned ${response.status}`]
    }

    /**
     * @param {Probe} probe
     * @param {string} body
     * @returns {string[]}
     */
    #contentFailure(probe, body) {
        return body.includes(probe.contains) ? [] : [`${probe.path} lacks ${JSON.stringify(probe.contains)}`]
    }

    /**
     * @param {Probe} probe
     * @param {string} contentType
     * @returns {string[]}
     */
    #typeFailure(probe, contentType) {
        return this.#typeAllowed(probe.contentTypes, contentType) ? [] : [`${probe.path} has content type ${contentType}`]
    }

    /**
     * @param {string[]} allowed
     * @param {string} contentType
     * @returns {boolean}
     */
    #typeAllowed(allowed, contentType) {
        if (allowed.length === 0) return true
        return this.#startsWithAny(contentType, allowed)
    }

    /**
     * @param {string} text
     * @param {string[]} prefixes
     * @returns {boolean}
     */
    #startsWithAny(text, prefixes) {
        return prefixes.some((prefix) => text.startsWith(prefix))
    }

    /**
     * @param {Response} response
     * @returns {string}
     */
    #contentType(response) {
        return response.headers.get('content-type') ?? ''
    }

    /**
     * @param {Probe} probe
     * @param {Headers} headers
     * @param {Record<string, string>} requiredHeaders
     * @returns {string[]}
     */
    #missingFromResponse(probe, headers, requiredHeaders) {
        return Object.entries(requiredHeaders)
            .filter(([name, value]) => headers.get(name) !== value)
            .map(([name]) => `${probe.path} lacks header ${name}`)
    }

    /**
     * @param {string} policy
     * @param {Record<string, string>} requiredHeaders
     * @returns {string[]}
     */
    #missingFromPolicy(policy, requiredHeaders) {
        const text = this.#collapseSpaces(policy).toLowerCase()
        return Object.entries(requiredHeaders)
            .filter((header) => !this.#policyHas(text, header))
            .map(([name]) => name)
    }

    /**
     * @param {string} text
     * @param {[string, string]} header
     * @returns {boolean}
     */
    #policyHas(text, [name, value]) {
        return text.includes(name.toLowerCase()) && text.includes(this.#collapseSpaces(value).toLowerCase())
    }

    /**
     * @param {string} text
     * @returns {string}
     */
    #collapseSpaces(text) {
        return text.replace(/\s+/g, ' ')
    }

    /**
     * @param {Response} response
     * @param {string} action
     */
    #requireOk(response, action) {
        if (!response.ok) throw new Error(`${action} failed with HTTP ${response.status}`)
    }

    /**
     * @param {string} actual
     * @param {string} expected
     * @param {string} subject
     */
    #requireEqual(actual, expected, subject) {
        if (actual !== expected) throw new Error(`${subject} is ${actual}, expected ${expected}`)
    }

    /**
     * @param {string[]} missing
     * @param {string} message
     */
    #requireNothingMissing(missing, message) {
        if (missing.length > 0) throw new Error(`${message}: ${missing.join('; ')}`)
    }

    /**
     * @param {string} command
     * @param {string[]} args
     * @param {RunOptions} [options]
     * @returns {string}
     */
    static run(command, args, options = {}) {
        const result = spawnSync(command, args, { cwd: options.cwd, input: options.input, env: { ...process.env, ...options.env }, encoding: 'utf8' })
        SiteDeploy.#requireSuccess(command, result)
        return result.stdout
    }

    /**
     * @param {string} command
     * @param {import('node:child_process').SpawnSyncReturns<string>} result
     */
    static #requireSuccess(command, result) {
        if (result.status !== 0) throw new Error(`${command} failed (${result.status ?? result.error?.message}): ${result.stderr}`)
    }

    /** @returns {SiteDeploy} */
    static live() {
        return new SiteDeploy({
            run: SiteDeploy.run,
            fetch: globalThis.fetch,
            sleep: (seconds) => new Promise((resolve) => setTimeout(resolve, seconds * 1000)),
            log: (line) => console.log(line),
            mask: SiteDeploy.#maskFor(process.env),
        })
    }

    /**
     * @param {Record<string, string | undefined>} environment
     * @returns {(secret: string) => void}
     */
    static #maskFor(environment) {
        return environment.GITHUB_ACTIONS === 'true' ? (secret) => console.log(`::add-mask::${secret}`) : () => {}
    }

    /**
     * @param {string} path
     * @returns {SiteConfig}
     */
    static config(path) {
        return JSON.parse(readFileSync(path, 'utf8'))
    }

    /**
     * @param {string[]} argv
     * @param {SiteDeploy} [site]
     */
    static async main(argv, site = SiteDeploy.live()) {
        const [command, ...args] = argv
        const output = await SiteDeploy.#command(site, command, args)
        console.log(output)
    }

    /**
     * @param {SiteDeploy} site
     * @param {string} command
     * @param {string[]} args
     * @returns {Promise<string>}
     */
    static async #command(site, command, args) {
        switch (command) {
            case 'package':
                return site.package(args[0], args[1], args[2], args[3])
            case 'verify':
                site.verify(args[0], args[1])
                return 'verified'
            case 'check-headers':
                site.checkHeaders(SiteDeploy.config(args[0]))
                return 'header policy covers the required headers'
            case 'deploy':
                await site.deploy(SiteDeploy.config(args[0]), args[1], args[2])
                return 'deployed'
            case 'live-version':
                return site.liveVersion(SiteDeploy.config(args[0]), args[1])
            case 'probe':
                await site.probe(SiteDeploy.config(args[0]), args[1], args[2])
                return 'probes passed'
            default:
                throw new Error('Usage: site-deploy.mjs package|verify|check-headers|deploy|live-version|probe ...')
        }
    }
}

if (import.meta.main) await SiteDeploy.main(process.argv.slice(2))
