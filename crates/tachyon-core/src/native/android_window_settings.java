  private static final boolean WINDOW_INTEGRATED = __WINDOW_INTEGRATED__;
  private static final boolean WINDOW_FULLSCREEN = __WINDOW_FULLSCREEN__;
  private static final boolean WINDOW_HAS_THEME_COLOR = __WINDOW_HAS_THEME_COLOR__;
  private static final int WINDOW_THEME_COLOR = __WINDOW_THEME_COLOR__;
  private static final boolean WINDOW_HAS_BACKGROUND_COLOR = __WINDOW_HAS_BACKGROUND_COLOR__;
  private static final int WINDOW_BACKGROUND_COLOR = __WINDOW_BACKGROUND_COLOR__;
  private static final boolean WINDOW_LIGHT_BARS = __WINDOW_LIGHT_BARS__;
  private volatile String titleBarInsets = "{\"height\":0,\"leading\":0,\"trailing\":0}";

  @SuppressWarnings("deprecation")
  private void installWindowSettings(FrameLayout root) {
    android.view.Window window = getWindow();
    if (WINDOW_HAS_BACKGROUND_COLOR) view.setBackgroundColor(WINDOW_BACKGROUND_COLOR);
    if (WINDOW_HAS_THEME_COLOR) {
      root.setBackgroundColor(WINDOW_THEME_COLOR);
      window.setNavigationBarColor(WINDOW_THEME_COLOR);
      window.setStatusBarColor(WINDOW_INTEGRATED ? android.graphics.Color.TRANSPARENT : WINDOW_THEME_COLOR);
    }
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
      window.setDecorFitsSystemWindows(false);
      android.view.WindowInsetsController controller = window.getInsetsController();
      if (controller != null) {
        int light = android.view.WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
            | android.view.WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS;
        if (WINDOW_HAS_THEME_COLOR) controller.setSystemBarsAppearance(WINDOW_LIGHT_BARS ? light : 0, light);
        if (WINDOW_FULLSCREEN) {
          controller.setSystemBarsBehavior(android.view.WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
          controller.hide(WindowInsets.Type.systemBars());
        }
      }
    } else {
      int flags = android.view.View.SYSTEM_UI_FLAG_LAYOUT_STABLE;
      if (WINDOW_HAS_THEME_COLOR && WINDOW_LIGHT_BARS) flags |= android.view.View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | android.view.View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
      if (WINDOW_INTEGRATED) flags |= android.view.View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN;
      if (WINDOW_FULLSCREEN) flags |= android.view.View.SYSTEM_UI_FLAG_FULLSCREEN | android.view.View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | android.view.View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
      window.getDecorView().setSystemUiVisibility(flags);
    }
    root.setOnApplyWindowInsetsListener(
        (padded, insets) -> {
          int[] edges = windowEdges(insets);
          padded.setPadding(edges[0], WINDOW_INTEGRATED ? 0 : edges[1], edges[2], edges[3]);
          publishTitleBarInsets(padded, edges);
          return insets;
        });
    root.requestApplyInsets();
  }

  @SuppressWarnings("deprecation")
  private static int[] windowEdges(WindowInsets insets) {
    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
      Insets bars = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
      return new int[] {bars.left, bars.top, bars.right, bars.bottom};
    }
    return new int[] {
      insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(),
      insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom()
    };
  }

  private void publishTitleBarInsets(android.view.View root, int[] edges) {
    float density = getResources().getDisplayMetrics().density;
    boolean rightToLeft = root.getLayoutDirection() == android.view.View.LAYOUT_DIRECTION_RTL;
    int leading = Math.round((rightToLeft ? edges[2] : edges[0]) / density);
    int trailing = Math.round((rightToLeft ? edges[0] : edges[2]) / density);
    int height = WINDOW_INTEGRATED ? Math.round(edges[1] / density) : 0;
    String answer = "{\"height\":" + height + ",\"leading\":" + leading + ",\"trailing\":" + trailing + "}";
    if (answer.equals(titleBarInsets)) return;
    titleBarInsets = answer;
    WebView web = view;
    if (WINDOW_INTEGRATED && web != null) web.evaluateJavascript("globalThis.__tachyonTitleBar?.apply(" + answer + ")", null);
  }
