@Service
export default class LinkService {
  /** @type {{ slug: string, title: string }} */
  topic = { slug: 'routing', title: 'Routing' }
  state = 'ready'
  message = 'Everything is up to date.'
}