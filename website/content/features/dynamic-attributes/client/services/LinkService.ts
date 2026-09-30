import type { Topic } from '/models/Topic.ts'

@Service
export default class LinkService {
  topic: Topic = { slug: 'routing', title: 'Routing' }
  state: string = 'ready'
  message: string = 'Everything is up to date.'
}
