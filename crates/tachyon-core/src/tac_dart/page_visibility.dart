final class PageVisibility {
  const PageVisibility();

  bool visible() => TacWebHost.call('pageVisibility', 'visible') as bool;
}
