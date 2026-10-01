function checkComposedServices(): void {
  const clipboard: ClipboardService = new ClipboardService();
  const label: string = clipboard.label('install');
  const choice: PlatformChoiceService = new PlatformChoiceService();
  const chosen: void = choice.choose('web');
  const feature: FeatureService = new FeatureService();
  const slug: string = feature.id;
  const featureSource: string = feature.source;
  const featureEntry: __TachyonKotlinModels.FeatureEntry = feature.entry();
  const compatibleEntry: {id: string; title: string; summary: string; group: string; files: Array<{name: string; code: string}>} = featureEntry;
  const files: Array<{name: string; code: string}> = feature.files();
  feature.routePath = '/docs/features/routing/';
  void slug; void featureSource; void compatibleEntry; void files;
  // @ts-expect-error Computed Kotlin val is readonly.
  feature.id = 'other';
  // @ts-expect-error Injected DocumentTitle does not become a public browser field.
  feature.documentTitle.set('hidden');
  // @ts-expect-error Lifecycle remains activation-owned.
  feature.initialize();
  // @ts-expect-error The actual page path is a String input.
  feature.routePath = 42;
  const topic: TopicService = new TopicService();
  const sections = topic.sections();
  const heading: string | undefined = sections[0]?.heading;
  void chosen;
  void label;
  void heading;

  // @ts-expect-error Composed names retain the source class's members.
  clipboard.nonexistentMethod();
  // @ts-expect-error The service still requires a string key.
  clipboard.label(42);
  // @ts-expect-error The Java facade retains its String ID contract.
  choice.choose(42);
  // @ts-expect-error Injected framework providers are not public service members.
  choice.applicationEventPublisher = { publishEvent: 'not a method' };
  // @ts-expect-error View contents retain their declared anchor type.
  const anchor: number = topic.contents()[0].anchor;
  void anchor;
}
void checkComposedServices;
