@Service
export default class MenuService {
  expanded: boolean = false;

  toggle(): void {
    this.expanded = !this.expanded;
  }

  close(): void {
    this.expanded = false;
  }
}
