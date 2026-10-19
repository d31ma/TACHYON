package dev.tachyon.host;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

public final class TachyonRust {
  public interface Host {
    void publish(String payload);

    void dispatch();

    String capability(String capability, String payload);
  }

  private static final String UNAVAILABLE = "{\"error\":\"The Rust services library did not load.\"}";
  private static final String UNSUPPORTED = "{\"status\":\"unsupported\"}";
  private static final int STATUS_BYTES = 4;
  private static final int BUFFER_BYTES = 16384;
  private static volatile Host host;
  private static volatile boolean loaded;

  private TachyonRust() {}

  public static synchronized boolean attach(Host value, String storeFolder) {
    if (loaded) return true;
    host = value;
    try {
      System.loadLibrary("__LIBRARY__");
      install(storeFolder.getBytes(StandardCharsets.UTF_8));
      loaded = true;
    } catch (UnsatisfiedLinkError error) {
      loaded = false;
    }
    return loaded;
  }

  public static String invoke(String request) {
    if (!loaded) return UNAVAILABLE;
    byte[] answer = call(request.getBytes(StandardCharsets.UTF_8));
    return answer == null ? UNAVAILABLE : new String(answer, StandardCharsets.UTF_8);
  }

  private static native void install(byte[] storeFolder);

  private static native byte[] call(byte[] request);

  private static void publish(byte[] payload) {
    Host current = host;
    if (current != null) current.publish(new String(payload, StandardCharsets.UTF_8));
  }

  private static void dispatch() {
    Host current = host;
    if (current != null) current.dispatch();
  }

  private static byte[] capability(byte[] capability, byte[] payload) {
    Host current = host;
    String answer = current == null ? UNSUPPORTED
        : current.capability(new String(capability, StandardCharsets.UTF_8),
            new String(payload, StandardCharsets.UTF_8));
    return (answer == null ? UNSUPPORTED : answer).getBytes(StandardCharsets.UTF_8);
  }

  private static byte[] http(byte[] method, byte[] url, byte[] headers, byte[] body, long timeoutMs, long limit) {
    long deadline = System.currentTimeMillis() + timeoutMs;
    HttpURLConnection connection = null;
    try {
      connection = (HttpURLConnection) new URL(new String(url, StandardCharsets.UTF_8)).openConnection();
      int timeout = (int) Math.max(1L, Math.min(Integer.MAX_VALUE, timeoutMs));
      connection.setConnectTimeout(timeout);
      connection.setReadTimeout(timeout);
      connection.setInstanceFollowRedirects(false);
      connection.setUseCaches(false);
      connection.setRequestMethod(new String(method, StandardCharsets.UTF_8));
      String[] lines = new String(headers, StandardCharsets.UTF_8).split("\n", -1);
      for (int index = 0; index + 1 < lines.length; index += 2) {
        connection.setRequestProperty(lines[index], lines[index + 1]);
      }
      if (body != null) {
        connection.setDoOutput(true);
        connection.setFixedLengthStreamingMode(body.length);
        try (OutputStream sent = connection.getOutputStream()) {
          sent.write(body);
        }
      }
      int status = connection.getResponseCode();
      InputStream stream = status >= 400 ? connection.getErrorStream() : connection.getInputStream();
      return answer(status, received(stream, limit, deadline));
    } catch (java.net.SocketTimeoutException error) {
      return failure("DEADLINE");
    } catch (HttpStop error) {
      return failure(error.getMessage());
    } catch (IllegalArgumentException | java.net.ProtocolException error) {
      return failure("HTTP_REQUEST");
    } catch (IOException | RuntimeException error) {
      return failure("HTTP_NETWORK");
    } finally {
      if (connection != null) connection.disconnect();
    }
  }

  private static byte[] received(InputStream stream, long limit, long deadline) throws IOException {
    ByteArrayOutputStream received = new ByteArrayOutputStream();
    if (stream == null) return received.toByteArray();
    try (InputStream reading = stream) {
      byte[] buffer = new byte[BUFFER_BYTES];
      for (int read = reading.read(buffer); read != -1; read = reading.read(buffer)) {
        if (received.size() + (long) read > limit) throw new HttpStop("HTTP_LIMIT");
        if (System.currentTimeMillis() > deadline) throw new HttpStop("DEADLINE");
        received.write(buffer, 0, read);
      }
    }
    return received.toByteArray();
  }

  private static byte[] answer(int status, byte[] body) {
    byte[] answer = new byte[STATUS_BYTES + body.length];
    answer[0] = (byte) (status >>> 24);
    answer[1] = (byte) (status >>> 16);
    answer[2] = (byte) (status >>> 8);
    answer[3] = (byte) status;
    System.arraycopy(body, 0, answer, STATUS_BYTES, body.length);
    return answer;
  }

  private static byte[] failure(String code) {
    return answer(0, code.getBytes(StandardCharsets.UTF_8));
  }

  private static final class HttpStop extends IOException {
    HttpStop(String code) {
      super(code);
    }
  }
}
