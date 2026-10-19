static int g_window_content_width = __WINDOW_WIDTH__;
static int g_window_content_height = __WINDOW_HEIGHT__;
static int g_window_frame_width = 0;
static int g_window_frame_height = 0;
static int g_window_frame_checks = 0;
static const int TACHYON_WINDOW_FRAME_CHECKS = 120;

static void tachyon_window_content_size(GtkWindow *window, int width, int height) {
  g_window_content_width = width;
  g_window_content_height = height;
  gtk_window_set_default_size(window, width + g_window_frame_width, height + g_window_frame_height);
}

static gboolean tachyon_window_frame_is_plausible(int frame_width, int frame_height) {
  return frame_width >= 0 && frame_width <= 200 && frame_height >= 0 && frame_height <= 200;
}

static gboolean tachyon_window_measure_frame(GtkWidget *content, GdkFrameClock *clock, gpointer data) {
  (void)clock;
  GtkWindow *window = GTK_WINDOW(data);
  if (gtk_window_is_fullscreen(window) || gtk_window_is_maximized(window)) return G_SOURCE_REMOVE;
  int width = gtk_widget_get_width(content);
  int height = gtk_widget_get_height(content);
  if (width <= 0 || height <= 0) return G_SOURCE_CONTINUE;
  if (width == g_window_content_width && height == g_window_content_height) return G_SOURCE_REMOVE;
  if (g_window_frame_checks == 0) {
    int frame_width = g_window_content_width - width;
    int frame_height = g_window_content_height - height;
    if (!tachyon_window_frame_is_plausible(frame_width, frame_height)) return G_SOURCE_REMOVE;
    g_window_frame_width = frame_width;
    g_window_frame_height = frame_height;
  }
  tachyon_window_content_size(window, g_window_content_width, g_window_content_height);
  g_window_frame_checks += 1;
  return g_window_frame_checks < TACHYON_WINDOW_FRAME_CHECKS ? G_SOURCE_CONTINUE : G_SOURCE_REMOVE;
}

static void tachyon_window_appearance(GtkWindow *window, GtkWidget *content) {
  gtk_window_set_default_size(window, __WINDOW_WIDTH__, __WINDOW_HEIGHT__);
  gtk_widget_add_tick_callback(content, tachyon_window_measure_frame, window, NULL);
  __WINDOW_MINIMUM_SIZE__
  __WINDOW_THEME_COLOR__
  __WINDOW_DISPLAY__
}
