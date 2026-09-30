@Service
export default class ItemService {
  idFrom(path: string): string { return path.split('/').pop() ?? '' }
}
