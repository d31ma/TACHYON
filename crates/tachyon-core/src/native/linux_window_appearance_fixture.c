#include <stdlib.h>

static int g_appearance_step = 0;
static gint64 g_appearance_started = 0;

static void tachyon_appearance_fail(const char *message) {
  int width = g_view != NULL ? gtk_widget_get_width(GTK_WIDGET(g_view)) : -1;
  int height = g_view != NULL ? gtk_widget_get_height(GTK_WIDGET(g_view)) : -1;
  fprintf(stderr, "linux window appearance fixture failed at step %d after %.1f s: %s (page %dx%d)\n", g_appearance_step,
          (double)(g_get_monotonic_time() - g_appearance_started) / G_USEC_PER_SEC, message, width, height);
  exit(1);
}

static void tachyon_appearance_require(gboolean valid, const char *message) {
  if (!valid) tachyon_appearance_fail(message);
}

static gboolean tachyon_appearance_page_loaded(void) {
  const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
  return g_strcmp0(uri, "tachyon://app/") == 0 && !webkit_web_view_is_loading(g_view) &&
         g_native_window != NULL && gtk_widget_get_mapped(GTK_WIDGET(g_native_window));
}

static gboolean tachyon_appearance_content_size(int width, int height) {
  GtkWidget *content = GTK_WIDGET(g_view);
  return gtk_widget_get_width(content) == width && gtk_widget_get_height(content) == height;
}

static gboolean tachyon_appearance_widget_pixel(GtkWidget *widget, guint red, guint green, guint blue) {
  int width = gtk_widget_get_width(widget);
  int height = gtk_widget_get_height(widget);
  if (width < 8 || height < 8) return FALSE;
  GdkPaintable *paintable = gtk_widget_paintable_new(widget);
  GtkSnapshot *snapshot = gtk_snapshot_new();
  gdk_paintable_snapshot(paintable, GDK_SNAPSHOT(snapshot), width, height);
  GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
  g_object_unref(paintable);
  if (node == NULL) return FALSE;
  graphene_rect_t bounds = GRAPHENE_RECT_INIT(0, 0, width, height);
  GdkTexture *texture = gsk_renderer_render_texture(gtk_native_get_renderer(GTK_NATIVE(g_native_window)), node, &bounds);
  gsk_render_node_unref(node);
  gsize stride = (gsize)width * 4;
  guchar *pixels = g_malloc(stride * (gsize)height);
  gdk_texture_download(texture, pixels, stride);
  g_object_unref(texture);
  const guchar *pixel = pixels + (gsize)(height / 2) * stride + (gsize)(width / 4) * 4;
  gboolean matches = abs((int)pixel[2] - (int)red) <= 2 && abs((int)pixel[1] - (int)green) <= 2 && abs((int)pixel[0] - (int)blue) <= 2;
  if (!matches) printf("%s pixel is %02x%02x%02x\n", G_OBJECT_TYPE_NAME(widget), pixel[2], pixel[1], pixel[0]);
  g_free(pixels);
  return matches;
}

static gboolean tachyon_appearance_header_pixel(guint red, guint green, guint blue) {
  return tachyon_appearance_widget_pixel(gtk_window_get_titlebar(g_native_window), red, green, blue);
}

static gboolean tachyon_appearance_page_background(void) {
  GdkRGBA color;
  webkit_web_view_get_background_color(g_view, &color);
  gboolean declared = abs((int)(color.red * 255 + 0.5) - 0xfd) <= 1 && abs((int)(color.green * 255 + 0.5) - 0xf6) <= 1 &&
                      abs((int)(color.blue * 255 + 0.5) - 0xe3) <= 1 && color.alpha > 0.99;
  tachyon_appearance_require(declared, "background_color is not the web view background");
  tachyon_appearance_require(gtk_widget_has_css_class(GTK_WIDGET(g_native_window), "tachyon-background"),
                             "background_color did not reach the window");
  return tachyon_appearance_widget_pixel(GTK_WIDGET(g_view), 0xfd, 0xf6, 0xe3);
}

static gboolean tachyon_appearance_header_text_is_white(void) {
  GdkRGBA color;
  gtk_widget_get_color(gtk_window_get_titlebar(g_native_window), &color);
  return color.red > 0.99 && color.green > 0.99 && color.blue > 0.99;
}

static gboolean tachyon_appearance_fullscreen(void) {
  GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(g_native_window));
  return gtk_window_is_fullscreen(g_native_window) && surface != NULL && GDK_IS_TOPLEVEL(surface) &&
         (gdk_toplevel_get_state(GDK_TOPLEVEL(surface)) & GDK_TOPLEVEL_STATE_FULLSCREEN) != 0;
}

static void tachyon_appearance_resize_below_minimum(void) {
  size_t length = 0;
  unsigned char *answer = tachyon_capability_call("window.resize", "{\"width\":300,\"height\":300}", &length);
  gboolean accepted = length == strlen("{\"ok\":true}") && memcmp(answer, "{\"ok\":true}", length) == 0;
  tachyon_capability_free(answer, length);
  tachyon_appearance_require(accepted, "the granted resize was refused");
}

static void tachyon_appearance_check_declared(void) {
  int width = 0, height = 0;
  gtk_widget_get_size_request(GTK_WIDGET(g_view), &width, &height);
  tachyon_appearance_require(width == 720 && height == 520, "the declared minimum size is not the page's size request");
  GtkWidget *header = gtk_window_get_titlebar(g_native_window);
  tachyon_appearance_require(header != NULL && GTK_IS_HEADER_BAR(header) && gtk_widget_has_css_class(header, "tachyon-theme"),
                             "theme_color did not install the header bar");
  tachyon_appearance_require(tachyon_appearance_header_text_is_white(), "the header bar text is not white on the dark theme_color");
  tachyon_appearance_require(!gtk_window_is_fullscreen(g_native_window), "a standalone window started fullscreen");
}

static gboolean tachyon_appearance_themed_tick(void) {
  switch (g_appearance_step) {
    case 0: if (!tachyon_appearance_page_loaded()) return FALSE; tachyon_appearance_check_declared(); return TRUE;
    case 1: return tachyon_appearance_content_size(900, 640);
    case 2: return tachyon_appearance_header_pixel(0x1a, 0x23, 0x7e);
    case 3: tachyon_appearance_resize_below_minimum(); return TRUE;
    case 4: return tachyon_appearance_content_size(720, 520);
    case 5: return tachyon_appearance_page_background();
    default: return FALSE;
  }
}

static gboolean tachyon_appearance_fullscreen_tick(void) {
  switch (g_appearance_step) {
    case 0:
      if (!tachyon_appearance_page_loaded()) return FALSE;
      tachyon_appearance_require(gtk_window_get_titlebar(g_native_window) == NULL, "a window without theme_color got a header bar");
      tachyon_appearance_require(!gtk_widget_has_css_class(GTK_WIDGET(g_native_window), "tachyon-background"),
                                 "a window without background_color got a background");
      return TRUE;
    case 1: return tachyon_appearance_fullscreen();
    default: return FALSE;
  }
}

static gboolean tachyon_appearance_tick(gpointer data) {
  (void)data;
  gboolean advanced = TACHYON_APPEARANCE_FULLSCREEN ? tachyon_appearance_fullscreen_tick() : tachyon_appearance_themed_tick();
  if (!advanced) return G_SOURCE_CONTINUE;
  g_appearance_step += 1;
  printf("step %d done after %.1f s\n", g_appearance_step, (double)(g_get_monotonic_time() - g_appearance_started) / G_USEC_PER_SEC);
  if (g_appearance_step == TACHYON_APPEARANCE_STEPS) {
    g_application_quit(g_application_get_default());
    return G_SOURCE_REMOVE;
  }
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_appearance_deadline(gpointer data) {
  (void)data;
  tachyon_appearance_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_appearance_started = g_get_monotonic_time();
  g_timeout_add(20, tachyon_appearance_tick, NULL);
  g_timeout_add_seconds(110, tachyon_appearance_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_appearance_require(status == 0 && g_appearance_step == TACHYON_APPEARANCE_STEPS, "the host did not finish its run");
  printf("linux window appearance passed: %s\n", TACHYON_APPEARANCE_FULLSCREEN
      ? "display fullscreen started the GTK window fullscreen without a header bar"
      : "the 900x640 size, the 720x520 minimum, the theme_color header bar and the background_color page backdrop reached the GTK window");
  return 0;
}
