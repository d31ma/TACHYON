@Setup
export default class ShelfSetup {
  static configure() {
    const execute = async (command: BehaviorJsonObject) => {
      const response = await fetch('/shelf', { method: 'POST', body: JSON.stringify(command) });
      return response.json();
    };
    return { authorizeEntity: (command: BehaviorJsonObject) => command.entity === 'BookEntity', entities: { shelf: { execute } } };
  }
}
