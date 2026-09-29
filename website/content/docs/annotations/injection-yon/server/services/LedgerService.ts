@Service
export default class LedgerService {
  @Needs
  repository!: LedgerStore;
  subtotal(): number { return this.repository.subtotal(); }
}
