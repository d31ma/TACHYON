  private static final String DEVELOPMENT_ORIGIN = "__DEVELOPMENT_ORIGIN__";

  private WebResourceResponse developmentResponse(String name, Uri target) {
    String query = target.getEncodedQuery();
    java.net.HttpURLConnection connection = null;
    try {
      java.net.URL address = new java.net.URL(
          DEVELOPMENT_ORIGIN + "/" + Uri.encode(name, "/") + (query != null ? "?" + query : ""));
      connection = (java.net.HttpURLConnection) address.openConnection();
      connection.setUseCaches(false);
      connection.setInstanceFollowRedirects(false);
      connection.setConnectTimeout(30000);
      connection.setReadTimeout(30000);
      int status = connection.getResponseCode();
      if (status >= 300 && status < 400) {
        return developmentFailure("The development server redirected " + name + ".");
      }
      InputStream stream = status >= 400 ? connection.getErrorStream() : connection.getInputStream();
      byte[] body = stream != null ? readLimited(stream) : new byte[0];
      String type = connection.getContentType();
      String mime = type != null ? type.split(";", 2)[0].trim() : assetMimeType(name);
      if (mime.equals("text/html")) body = withShim(body);
      String reason = connection.getResponseMessage();
      java.util.Map<String, String> headers = new java.util.HashMap<>();
      headers.put("Content-Security-Policy", CONTENT_SECURITY_POLICY);
      headers.put("Cache-Control", "no-store");
      return new WebResourceResponse(mime, "UTF-8", status, reason != null && !reason.isEmpty() ? reason : "OK",
          headers, new ByteArrayInputStream(body));
    } catch (Exception error) {
      return developmentFailure("The development server did not answer: " + error.getMessage());
    } finally {
      if (connection != null) connection.disconnect();
    }
  }

  private static WebResourceResponse developmentFailure(String reason) {
    java.util.Map<String, String> headers = new java.util.HashMap<>();
    headers.put("Cache-Control", "no-store");
    return new WebResourceResponse("text/plain", "UTF-8", 502, "Bad Gateway", headers,
        new ByteArrayInputStream(reason.getBytes(StandardCharsets.UTF_8)));
  }
