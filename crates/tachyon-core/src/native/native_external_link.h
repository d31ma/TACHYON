#define TACHYON_EXTERNAL_LINK_LIMIT 8192

static int tachyon_link_prefix_matches(const char *uri, const char *prefix) {
  for (size_t index = 0; prefix[index] != '\0'; index++) {
    char ch = uri[index], expected = prefix[index];
    if (ch >= 'A' && ch <= 'Z') ch = (char)(ch - 'A' + 'a');
    if (expected >= 'A' && expected <= 'Z') expected = (char)(expected - 'A' + 'a');
    if (ch != expected) return 0;
  }
  return 1;
}

static int tachyon_link_on_local_origin(const char *uri) {
  size_t length = strlen(TACHYON_LOCAL_ORIGIN);
  if (!tachyon_link_prefix_matches(uri, TACHYON_LOCAL_ORIGIN)) return 0;
  char next = uri[length];
  return next == '\0' || next == '/' || next == '?' || next == '#' || next == ':';
}

static int tachyon_external_link(const char *uri) {
  if (uri == NULL) return 0;
  size_t length = strnlen(uri, TACHYON_EXTERNAL_LINK_LIMIT + 1);
  if (length == 0 || length > TACHYON_EXTERNAL_LINK_LIMIT) return 0;
  for (size_t index = 0; index < length; index++) {
    unsigned char ch = (unsigned char)uri[index];
    if (ch <= ' ' || ch >= 127 || ch == '"') return 0;
  }
  if (tachyon_link_on_local_origin(uri)) return 0;
  if (tachyon_link_prefix_matches(uri, "mailto:")) return length > strlen("mailto:");
  size_t authority = 0;
  if (tachyon_link_prefix_matches(uri, "https://")) authority = strlen("https://");
  else if (tachyon_link_prefix_matches(uri, "http://")) authority = strlen("http://");
  else return 0;
  size_t host = strcspn(uri + authority, "/?#");
  return host > 0 && memchr(uri + authority, '@', host) == NULL;
}
