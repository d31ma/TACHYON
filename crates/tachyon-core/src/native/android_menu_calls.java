  private boolean menuReady(String event) {
    return dev.tachyon.host.TachyonRust.menuReady(event);
  }

  private void menuPerform(MenuRoute entry) {
    dev.tachyon.host.TachyonRust.menuAction(entry.id,
        performed -> runOnUiThread(() -> record(performed ? "menu.action" : "menu.refused")));
  }
