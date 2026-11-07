#define TACHYON_DEEP_LINK_LIMIT 2048
static const char *const TACHYON_LINK_SCHEMES[] = {__LINK_SCHEMES__NULL};
static const char *const TACHYON_LINK_HOSTS[] = {__LINK_HOSTS__NULL};

static int tachyon_link_listed(const char *const *names, const char *text, size_t length) {
  for (size_t index = 0; names[index] != NULL; index++) {
    if (strlen(names[index]) == length && tachyon_link_prefix_matches(text, names[index])) return 1;
  }
  return 0;
}

static const char *tachyon_deep_link_rest(const char *link) {
  size_t length = strnlen(link, TACHYON_DEEP_LINK_LIMIT + 1);
  if (length == 0 || length > TACHYON_DEEP_LINK_LIMIT) return NULL;
  for (size_t index = 0; index < length; index++) {
    unsigned char ch = (unsigned char)link[index];
    if (ch <= ' ' || ch >= 127 || ch == '"' || ch == '\\') return NULL;
  }
  const char *colon = strchr(link, ':');
  if (colon == NULL || colon == link) return NULL;
  if (tachyon_link_listed(TACHYON_LINK_SCHEMES, link, (size_t)(colon - link))) {
    const char *rest = colon + 1;
    while (*rest == '/') rest++;
    return rest;
  }
  if (!tachyon_link_prefix_matches(link, "https://")) return NULL;
  const char *authority = link + strlen("https://");
  size_t host = strcspn(authority, "/?#");
  if (!tachyon_link_listed(TACHYON_LINK_HOSTS, authority, host)) return NULL;
  return authority[host] == '/' ? authority + host + 1 : authority + host;
}

static int tachyon_deep_link_target(const char *link, char *target, size_t capacity) {
  const char *rest = link != NULL ? tachyon_deep_link_rest(link) : NULL;
  if (rest == NULL) return 0;
  size_t path = strcspn(rest, "?#");
  const char *suffix = rest + path;
  while (path > 0 && rest[path - 1] == '/') path--;
  char candidate[TACHYON_PATH_LIMIT], decoded[TACHYON_PATH_LIMIT];
  int written = snprintf(candidate, sizeof(candidate), "%s/%.*s%s", TACHYON_LOCAL_ORIGIN, (int)path, rest,
                         path > 0 ? "/" : "");
  int known = written > 0 && (size_t)written < sizeof(candidate) &&
              tachyon_local_path(candidate, decoded, sizeof(decoded)) && tachyon_document_route(decoded) != NULL;
  size_t entry = strlen(TACHYON_ENTRY_ROUTE);
  written = known ? snprintf(target, capacity, "%s%s", candidate, suffix)
                  : snprintf(target, capacity, "%s%s%s", TACHYON_LOCAL_ORIGIN, TACHYON_ENTRY_ROUTE,
                             entry > 0 && TACHYON_ENTRY_ROUTE[entry - 1] == '/' ? "" : "/");
  return written > 0 && (size_t)written < capacity;
}
