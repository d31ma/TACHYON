import { describe, expect, test } from 'bun:test'
import { TypeCheckProjects } from '../scripts/type-check-projects.mjs'
import { SiteContent } from '../scripts/site-content.mjs'

describe('tsc checks every JavaScript and TypeScript example project', () => {
  test('each language keeps its own sources and shares the models it lacks', () => {
    const files = [
      { name: 'server/controllers/GreetingController.js', code: '' },
      { name: 'server/controllers/GreetingController.ts', code: '' },
      { name: 'server/controllers/GreetingController.py', code: '' },
      { name: 'server/models/Greeting.ts', code: '' },
      { name: 'server/application.json', code: '{}' },
    ]
    /** @param {string} language */
    const names = language => TypeCheckProjects.filesFor(files, language).map(file => file.name)
    expect(names('.js')).toEqual(['server/controllers/GreetingController.js', 'server/models/Greeting.ts', 'server/application.json'])
    expect(names('.ts')).toEqual(['server/controllers/GreetingController.ts', 'server/models/Greeting.ts', 'server/application.json'])
  })

  test('a project is checked only in the realms that hold its language', () => {
    const example = {
      name: 'sockets',
      label: 'features/sockets',
      files: [
        { name: 'client/services/ChatService.js', code: '' },
        { name: 'server/models/ChatMessage.ts', code: '' },
      ],
    }
    const projects = TypeCheckProjects.projectsFor(example)
    expect(projects.map(project => [project.label, project.realms])).toEqual([
      ['features/sockets [js]', ['client']],
      ['features/sockets [ts]', ['server']],
    ])
  })

  test('tutorial steps build on the files of the steps before them', () => {
    const docs = {
      topics: {
        'first-app': {
          sections: [
            { heading: 'Step 1', files: [{ name: 'client/services/CounterService.js', code: 'one' }] },
            { heading: 'Step 2', files: [{ name: 'client/services/CounterService.js', code: 'two' }, { name: 'client/tests/CounterServiceTest.js', code: '' }] },
          ],
        },
      },
    }
    const [, second] = TypeCheckProjects.guideExamples(docs, new Set(['first-app']))
    expect(second.files.map(file => [file.name, file.code])).toEqual([
      ['client/services/CounterService.js', 'two'],
      ['client/tests/CounterServiceTest.js', ''],
    ])
  })

  test('the published site holds JavaScript and TypeScript projects in both realms', () => {
    const projects = TypeCheckProjects.collect(
      SiteContent.data('features.json').features,
      SiteContent.data('languages.json'),
      SiteContent.data('docs.json'),
      new Set(['first-app']),
    )
    const labels = projects.map(project => project.label)
    expect(labels).toContain('docs/first-app / Step 4: Call the server from the page [ts]')
    expect(labels).toContain('features/sockets [js]')
    expect(projects.some(project => project.realms.length === 2)).toBe(true)
  })

  test('filters keep only the projects of the chosen example and language', () => {
    const projects = [
      { name: 'sockets', label: 'features/sockets [js]', extension: '.js', realms: ['client'], files: [] },
      { name: 'sockets', label: 'features/sockets [ts]', extension: '.ts', realms: ['server'], files: [] },
      { name: 'routing', label: 'features/routing [ts]', extension: '.ts', realms: ['server'], files: [] },
    ]
    /** @param {{name?:string,language?:string}} filter */
    const labels = filter => TypeCheckProjects.select(projects, filter).map(project => project.label)
    expect(labels({ name: 'sockets', language: 'ts' })).toEqual(['features/sockets [ts]'])
    expect(labels({ language: 'js' })).toEqual(['features/sockets [js]'])
    expect(labels({})).toHaveLength(3)
  })

  test('a filter that leaves no project skips the tsc step instead of failing', () => {
    const projects = TypeCheckProjects.collect(
      SiteContent.data('features.json').features,
      SiteContent.data('languages.json'),
      SiteContent.data('docs.json'),
      new Set(['first-app']),
    )
    expect(TypeCheckProjects.select(projects, { name: 'Choose a service language' })).toEqual([])
    expect(TypeCheckProjects.select(projects, { language: 'py' })).toEqual([])
  })

  test('an unfiltered run with no project still fails', () => {
    expect(() => TypeCheckProjects.select([], {})).toThrow('No JavaScript or TypeScript example project')
  })
})
