  private Object systemBack;
  private boolean systemBackHeld;

  private final class SystemBack implements android.window.OnBackInvokedCallback {
    @Override
    public void onBackInvoked() {
      goBackInPage();
    }
  }

  private boolean goBackInPage() {
    WebView web = view;
    if (web == null || !web.canGoBack()) return false;
    record("navigation.back");
    web.goBack();
    return true;
  }

  private void holdSystemBack() {
    WebView web = view;
    if (android.os.Build.VERSION.SDK_INT < 33 || web == null) return;
    boolean wanted = web.canGoBack();
    if (wanted == systemBackHeld) return;
    if (systemBack == null) systemBack = new SystemBack();
    android.window.OnBackInvokedCallback callback = (android.window.OnBackInvokedCallback) systemBack;
    if (wanted) getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
        android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT, callback);
    else getOnBackInvokedDispatcher().unregisterOnBackInvokedCallback(callback);
    systemBackHeld = wanted;
  }

  @Override
  public void onBackPressed() {
    if (!goBackInPage()) super.onBackPressed();
  }
