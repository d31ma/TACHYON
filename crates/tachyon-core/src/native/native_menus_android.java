  private static final class MenuRoute {
    final String id;
    final String label;
    final String group;
    final String route;
    final String key;
    final int modifiers;
    final String event;
    final boolean action;

    MenuRoute(String id, String label, String group, String route, String key, int modifiers) {
      this(id, label, group, route, key, modifiers, "", false);
    }

    private MenuRoute(String id, String label, String group, String route, String key, int modifiers,
        String event, boolean action) {
      this.id = id;
      this.label = label;
      this.group = group;
      this.route = route;
      this.key = key;
      this.modifiers = modifiers;
      this.event = event;
      this.action = action;
    }

    static MenuRoute action(String id, String label, String group, String event, String key, int modifiers) {
      return new MenuRoute(id, label, group, "", key, modifiers, event, true);
    }

    boolean matches(KeyEvent event) {
      if (key.isEmpty() || event.getRepeatCount() != 0) return false;
      return event.getKeyCode() == keyCode() && event.hasModifiers(modifiers);
    }

    int keyCode() {
      char character = key.charAt(0);
      if (Character.isDigit(character)) return KeyEvent.KEYCODE_0 + (character - '0');
      return KeyEvent.KEYCODE_A + (character - 'a');
    }
  }

  private static final MenuRoute[] MENU_ROUTES = __NATIVE_MENU_ROUTES__;
  private static final String[] MENU_GROUPS = {"file", "view", "help"};
  private static final String[] MENU_GROUP_TITLES = {"File", "View", "Help"};

  private android.view.View withMenuBar(android.view.View content) {
    if (MENU_ROUTES.length == 0) return content;
    android.widget.Toolbar toolbar = new android.widget.Toolbar(this);
    toolbar.setTitle(getTitle());
    android.widget.LinearLayout column = new android.widget.LinearLayout(this);
    column.setOrientation(android.widget.LinearLayout.VERTICAL);
    column.addView(toolbar, new android.widget.LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
    column.addView(content, new android.widget.LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
    setActionBar(toolbar);
    return column;
  }

  @Override
  public boolean onCreateOptionsMenu(android.view.Menu menu) {
    for (int index = 0; index < MENU_ROUTES.length; index++) {
      MenuRoute entry = MENU_ROUTES[index];
      int group = java.util.Arrays.asList(MENU_GROUPS).indexOf(entry.group) + 1;
      android.view.MenuItem item = menu.add(group, index + 1, index, entry.label);
      item.setShowAsAction(android.view.MenuItem.SHOW_AS_ACTION_NEVER);
      if (!entry.key.isEmpty()) item.setAlphabeticShortcut(entry.key.charAt(0), entry.modifiers);
    }
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.P) menu.setGroupDividerEnabled(true);
    return MENU_ROUTES.length > 0;
  }

  @Override
  public boolean onPrepareOptionsMenu(android.view.Menu menu) {
    for (int index = 0; index < MENU_ROUTES.length; index++) {
      android.view.MenuItem item = menu.findItem(index + 1);
      if (item != null && MENU_ROUTES[index].action) item.setEnabled(menuActionEnabled(MENU_ROUTES[index]));
    }
    return MENU_ROUTES.length > 0;
  }

  @Override
  public boolean onOptionsItemSelected(android.view.MenuItem item) {
    int index = item.getItemId() - 1;
    if (index < 0 || index >= MENU_ROUTES.length) return super.onOptionsItemSelected(item);
    chooseMenuItem(MENU_ROUTES[index]);
    return true;
  }

  @Override
  public boolean dispatchKeyEvent(KeyEvent event) {
    MenuRoute shortcut = menuShortcut(event);
    if (shortcut == null) return super.dispatchKeyEvent(event);
    if (event.getAction() == KeyEvent.ACTION_DOWN) chooseMenuItem(shortcut);
    return true;
  }

  private void chooseMenuItem(MenuRoute entry) {
    if (!entry.action) {
      openMenuRoute(entry);
      return;
    }
    if (!menuActionEnabled(entry)) {
      record("menu.refused");
      return;
    }
    menuPerform(entry);
  }

  private boolean menuActionEnabled(MenuRoute entry) {
    return MENU_ROUTES.length <= 64 && view != null && (entry.event.isEmpty() || menuReady(entry.event));
  }

__NATIVE_MENU_CALLS__

  @Override
  public void onProvideKeyboardShortcuts(java.util.List<android.view.KeyboardShortcutGroup> data,
      android.view.Menu menu, int deviceId) {
    for (int group = 0; group < MENU_GROUPS.length; group++) {
      android.view.KeyboardShortcutGroup shortcuts =
          new android.view.KeyboardShortcutGroup(MENU_GROUP_TITLES[group]);
      for (MenuRoute entry : MENU_ROUTES) {
        if (!entry.group.equals(MENU_GROUPS[group]) || entry.key.isEmpty()) continue;
        shortcuts.addItem(new android.view.KeyboardShortcutInfo(entry.label, entry.keyCode(), entry.modifiers));
      }
      if (!shortcuts.getItems().isEmpty()) data.add(shortcuts);
    }
  }

  private MenuRoute menuShortcut(KeyEvent event) {
    for (MenuRoute entry : MENU_ROUTES) {
      if (entry.matches(event)) return entry;
    }
    return null;
  }

  private void openMenuRoute(MenuRoute entry) {
    if (view == null || canonicalRoute(entry.route) == null) return;
    record("menu.opened");
    view.loadUrl(routeTarget(entry.route.replaceAll("^/+|/+$", ""), ""));
  }
