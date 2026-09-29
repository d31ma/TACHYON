import path from 'node:path'

/** @typedef {{name:string,code:string}} Snippet */
/** @typedef {{id:string,files:Snippet[]}} Example */
/** @typedef {{heading:string,files?:Snippet[]}} ExampleSection */
/** @typedef {{name:string,label:string,files:Snippet[]}} ExampleProject */
/** @typedef {{name:string,label:string,extension:string,realms:string[],files:Snippet[]}} TypeCheckProject */

export class TypeCheckProjects {
  static extensions = ['.js', '.ts']
  static realms = ['client', 'server']
  static otherLanguageSource = /\.(py|php|java|groovy|kt|cs|fs|rs|swift|dart|go|rb)$/

  /**
   * @param {Example[]} features
   * @param {{tac:{files:Snippet[]},yon:{entries:Example[]}}} languages
   * @param {{topics:Record<string,{sections?:ExampleSection[]}>}} docs
   * @param {Set<string>} tutorials
   * @returns {TypeCheckProject[]}
   */
  static collect(features, languages, docs, tutorials) {
    const examples = [
      ...features.map(feature => ({ name: feature.id, label: `features/${feature.id}`, files: feature.files })),
      { name: 'tac', label: 'languages/tac', files: languages.tac.files },
      ...languages.yon.entries.map(entry => ({ name: entry.id, label: `languages/yon/${entry.id}`, files: entry.files })),
      ...TypeCheckProjects.guideExamples(docs, tutorials),
    ]
    return examples.flatMap(example => TypeCheckProjects.projectsFor(example))
  }

  /**
   * @param {{topics:Record<string,{sections?:ExampleSection[]}>}} docs
   * @param {Set<string>} tutorials
   * @returns {ExampleProject[]}
   */
  static guideExamples(docs, tutorials) {
    const examples = []
    for (const [slug, topic] of Object.entries(docs.topics)) {
      /** @type {Map<string, Snippet>} */
      const tutorialFiles = new Map()
      for (const section of topic.sections ?? []) {
        const files = section.files ?? []
        if (tutorials.has(slug)) for (const file of files) tutorialFiles.set(file.name, file)
        const projectFiles = tutorials.has(slug) ? [...tutorialFiles.values()] : files
        examples.push({ name: section.heading, label: `docs/${slug} / ${section.heading}`, files: projectFiles })
      }
    }
    return examples
  }

  /**
   * @param {ExampleProject} example
   * @returns {TypeCheckProject[]}
   */
  static projectsFor(example) {
    const projects = []
    for (const extension of TypeCheckProjects.extensions) {
      const files = TypeCheckProjects.filesFor(example.files, extension)
      const realms = TypeCheckProjects.realmsWith(files, extension)
      if (realms.length === 0) continue
      projects.push({ name: example.name, label: `${example.label} [${extension.slice(1)}]`, extension, realms, files })
    }
    return projects
  }

  /**
   * @param {Snippet[]} files
   * @param {string} extension
   * @returns {Snippet[]}
   */
  static filesFor(files, extension) {
    const ownStems = new Set(files.filter(file => path.extname(file.name) === extension).map(TypeCheckProjects.stem))
    return files.filter(file => {
      const fileExtension = path.extname(file.name)
      if (fileExtension === extension) return true
      if (TypeCheckProjects.extensions.includes(fileExtension)) return !ownStems.has(TypeCheckProjects.stem(file))
      return !TypeCheckProjects.otherLanguageSource.test(file.name)
    })
  }

  /**
   * @param {Snippet[]} files
   * @param {string} extension
   * @returns {string[]}
   */
  static realmsWith(files, extension) {
    return TypeCheckProjects.realms.filter(realm => files.some(file => {
      return file.name.startsWith(`${realm}/`) && path.extname(file.name) === extension
    }))
  }

  /**
   * @param {Snippet} file
   * @returns {string}
   */
  static stem(file) {
    return file.name.slice(0, -path.extname(file.name).length)
  }
}
