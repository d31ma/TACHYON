@Service
export default class MenuService {
  expanded = false

  toggle() { this.expanded = !this.expanded }
}
