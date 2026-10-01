function checkComposedServices(): void {
  const clipboard: ClipboardService = new ClipboardService();
  const label: string = clipboard.label('install');
  const choice: PlatformChoiceService = new PlatformChoiceService();
  choice.choose('web');
  const topic: TopicService = new TopicService();
  const sections = topic.sections();
  const heading: string | undefined = sections[0]?.heading;
  void label;
  void heading;

  // @ts-expect-error Composed names retain the source class's members.
  clipboard.nonexistentMethod();
  // @ts-expect-error The service still requires a string key.
  clipboard.label(42);
  // @ts-expect-error Injected framework providers are not public service members.
  choice.applicationEventPublisher = { publishEvent: 'not a method' };
  // @ts-expect-error View contents retain their declared anchor type.
  const anchor: number = topic.contents()[0].anchor;
  void anchor;
}
void checkComposedServices;
