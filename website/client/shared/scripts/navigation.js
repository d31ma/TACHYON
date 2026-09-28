// @ts-nocheck
import docs from '/shared/data/docs.json' with { type: 'json' }
import catalogue from '/shared/data/features.json' with { type: 'json' }

export function slugify(value) {
  return String(value)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
}

export function guides() {
  return docs.order.map((slug) => ({
    slug,
    title: docs.topics[slug].title,
    summary: docs.topics[slug].summary,
    path: `/docs/${slug}`,
  }))
}

export function guide(slug) {
  return docs.topics[slug]
}

export function redirect(slug, hash = '') {
  const target = docs.redirects?.[slug]
  if (!target) return undefined
  const anchor = hash.replace(/^#/, '')
  const holdsAnchor = (key) => docs.topics[key].sections.some((section) => slugify(section.heading) === anchor)
  const holder = anchor ? docs.order.find(holdsAnchor) : undefined
  return `/docs/${holder ?? target}${hash}`
}

export function features() {
  return catalogue.features.map((feature) => ({ ...feature, path: `/docs/features/${feature.id}` }))
}

export function feature(id) {
  return features().find((entry) => entry.id === id)
}

function joinText(parts) {
  return parts.filter(Boolean).join('\n')
}

function codeOf(files) {
  return (files ?? []).map((file) => file.code)
}

function searchRows() {
  const rows = []
  for (const entry of guides()) {
    const topic = guide(entry.slug)
    rows.push({ title: entry.title, path: entry.path, kind: 'topic', text: entry.summary })
    for (const section of topic.sections ?? []) {
      const anchor = slugify(section.heading)
      rows.push({
        title: `${entry.title} · ${section.heading}`,
        path: `${entry.path}#${anchor}`,
        kind: 'section',
        text: joinText([section.body, section.code, ...codeOf(section.files)]),
      })
    }
  }
  for (const entry of features()) {
    rows.push({
      title: entry.title,
      path: entry.path,
      kind: 'feature',
      text: joinText([entry.summary, entry.group, ...codeOf(entry.files)]),
    })
  }
  return rows
}

function matchRank(row, searchTerm) {
  const title = row.title.toLowerCase()
  if (title.startsWith(searchTerm)) return 3
  if (title.includes(searchTerm)) return 2
  const text = String(row.text ?? '').toLowerCase()
  if (text.includes(searchTerm)) return 1
  return 0
}

function byRankThenShorterTitle(left, right) {
  return right.rank - left.rank || left.row.title.length - right.row.title.length
}

export const searchDocs = (query, maximum = 12) => {
  const searchTerm = String(query ?? '').trim().toLowerCase()
  if (searchTerm.length < 2) return []
  const hits = []
  for (const row of searchRows()) {
    const rank = matchRank(row, searchTerm)
    if (rank > 0) hits.push({ row, rank })
  }
  hits.sort(byRankThenShorterTitle)
  const bestHits = hits.slice(0, maximum)
  return bestHits.map(({ row }) => ({ title: row.title, path: row.path, kind: row.kind }))
}

export function groups() {
  const result = []
  for (const name of catalogue.groups) {
    const members = features().filter((entry) => entry.group === name)
    if (members.length > 0) result.push({ name, slug: slugify(name), features: members })
  }
  return result
}

function titleAndPath(entry) {
  return { title: entry.title, path: entry.path }
}

export function reading() {
  const pages = guides().map(titleAndPath)
  for (const group of groups()) pages.push(...group.features.map(titleAndPath))
  return pages
}

export function neighbours(path) {
  const order = reading()
  const index = order.findIndex((entry) => entry.path === path)
  return {
    previous: index > 0 ? order[index - 1] : null,
    next: index >= 0 && index < order.length - 1 ? order[index + 1] : null,
  }
}

export function currentPath() {
  return location.pathname.replace(/(.)\/+$/, '$1')
}

export function currentSlug() {
  return currentPath().split('/').filter(Boolean).pop() ?? ''
}
