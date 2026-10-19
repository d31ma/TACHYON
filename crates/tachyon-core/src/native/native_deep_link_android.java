  private static final String[] LINK_SCHEMES = __LINK_SCHEMES__;
  private static final String[] LINK_HOSTS = __LINK_HOSTS__;

  static String linkTarget(String link) {
    String[] parts = linkParts(link);
    return parts == null ? null : routeTarget(parts[0], parts[1]);
  }

  static String routeTarget(String path, String suffix) {
    if (canonicalRoute("/" + path) == null) return ASSET_ORIGIN + ENTRY_DOCUMENT;
    return ASSET_ORIGIN + path + suffix;
  }

  private static boolean linkListed(String[] names, String text) {
    for (String name : names) {
      if (name.equalsIgnoreCase(text)) return true;
    }
    return false;
  }

  private static String[] linkParts(String link) {
    if (link == null || link.isEmpty() || link.length() > 2048) return null;
    for (int index = 0; index < link.length(); index++) {
      char ch = link.charAt(index);
      if (ch <= ' ' || ch >= 127 || ch == '"' || ch == '\\') return null;
    }
    int colon = link.indexOf(':');
    if (colon <= 0) return null;
    String rest = link.substring(colon + 1);
    if (linkListed(LINK_SCHEMES, link.substring(0, colon))) {
      rest = rest.replaceFirst("^/+", "");
    } else if (link.regionMatches(true, 0, "https://", 0, 8)) {
      String authority = link.substring(8);
      int host = 0;
      while (host < authority.length() && "/?#".indexOf(authority.charAt(host)) < 0) host++;
      if (!linkListed(LINK_HOSTS, authority.substring(0, host))) return null;
      rest = authority.substring(host);
      if (rest.startsWith("/")) rest = rest.substring(1);
    } else {
      return null;
    }
    int path = 0;
    while (path < rest.length() && "?#".indexOf(rest.charAt(path)) < 0) path++;
    String suffix = rest.substring(path);
    String route = rest.substring(0, path).replaceFirst("/+$", "");
    return new String[] {route, suffix};
  }
