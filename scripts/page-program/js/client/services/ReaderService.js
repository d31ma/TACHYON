@Service
export default class ReaderService {
  static _NOTE = 'note';
  @Needs
  lifecycle;
  /** @type {ProgramNoteStore} */
  @Needs
  programNoteStore;
  _client = null;
  identity = '';
  count = 0;
  recalled = '';

  @OnStart
  async open() {
    const behaviors = await Behaviors.create(PageProgramSetup, { partition: 'public-reader', signal: this.lifecycle });
    this._client = Object.assign(new PageProgramClient(), { behaviors });
  }

  async inspect() {
    this._show(await this._client.read());
  }

  async increment() {
    this._show(await this._client.increment());
  }

  async remember(text) {
    await this.programNoteStore.save({ id: ReaderService._NOTE, text });
  }

  async recall() {
    const note = await this.programNoteStore.findById(ReaderService._NOTE);
    this.recalled = this.#text(note);
  }

  #text(note) {
    return note?.text ?? '';
  }

  _show(state) {
    this.identity = state.identity;
    this.count = state.count;
  }
}
