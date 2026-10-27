@Setup
export default class ShelfSetup {
  /** @returns {object} */
  static configure() {
    /** @param {BehaviorJsonObject} command */
    const execute = async (command) => {
      const response = await fetch('/shelf', { method: 'POST', body: JSON.stringify(command) });
      return response.json();
    };
    return { authorizeEntity: (/** @type {BehaviorJsonObject} */ command) => command.entity === 'BookEntity', entities: { shelf: { execute } } };
  }
}
