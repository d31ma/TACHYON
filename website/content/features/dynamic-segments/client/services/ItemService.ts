@Service
export default class ItemService {
  idFrom(path: string): string { return path.slice(path.lastIndexOf('/') + 1) }
}
