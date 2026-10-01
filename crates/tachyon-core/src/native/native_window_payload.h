#include <stdint.h>

static size_t tachyon_window_field(const char *json, const char *key) {
  int depth = 0; size_t found = 0, key_length = strlen(key);
  for (size_t index = 0; json[index] != '\0' && index <= 65536; index++) {
    if (json[index] == '{' || json[index] == '[') depth++;
    else if (json[index] == '}' || json[index] == ']') { if (--depth < 0) return 0; }
    else if (json[index] == '"') {
      size_t start = ++index; int escaped = 0;
      for (; json[index] != '\0'; index++) {
        if (json[index] == '\\') { escaped = 1; if (json[++index] == '\0') return 0; }
        else if (json[index] == '"') break;
      }
      if (json[index] == '\0') return 0;
      size_t next = tachyon_skip_space(json, index + 1);
      if (depth == 1 && json[next] == ':' && !escaped && index - start == key_length && memcmp(json + start, key, key_length) == 0) {
        if (found) return 0;
        found = tachyon_skip_space(json, next + 1);
      }
    }
  }
  return depth == 0 ? found : 0;
}

static int tachyon_window_size(const char *json, const char *key) {
  size_t index = tachyon_window_field(json, key); int value = 0, count = 0;
  if (!index || json[index] == '0') return 0;
  for (; json[index] >= '0' && json[index] <= '9'; index++) {
    if (++count > 5) return 0;
    value = value * 10 + json[index] - '0';
  }
  index = tachyon_skip_space(json, index);
  return count && (json[index] == ',' || json[index] == '}') && value >= 64 && value <= 16384 ? value : 0;
}

static int tachyon_window_hex(const char *json, size_t *index, uint32_t *value) {
  *value = 0;
  for (int digit = 0; digit < 4; digit++) {
    unsigned char ch = (unsigned char)json[(*index)++];
    if (!ch) return 0;
    int code = ch >= '0' && ch <= '9' ? ch - '0' : ch >= 'a' && ch <= 'f' ? ch - 'a' + 10 : ch >= 'A' && ch <= 'F' ? ch - 'A' + 10 : -1;
    if (code < 0) return 0;
    *value = *value * 16 + (uint32_t)code;
  }
  return 1;
}

static int tachyon_window_title(const char *json, uint16_t *output, size_t capacity) {
  size_t index = tachyon_window_field(json, "title"), used = 0;
  if (!index || json[index++] != '"') return 0;
  for (; json[index] != '"';) {
    uint32_t code = (unsigned char)json[index++];
    if (!code || code < 32) return 0;
    if (code == '\\') {
      unsigned char escaped = (unsigned char)json[index++];
      switch (escaped) {
        case '"': case '\\': case '/': code = escaped; break;
        case 'b': code = 8; break; case 'f': code = 12; break;
        case 'n': code = 10; break; case 'r': code = 13; break; case 't': code = 9; break;
        case 'u':
          if (!tachyon_window_hex(json, &index, &code) || code == 0) return 0;
          if (code >= 0xD800 && code <= 0xDBFF) {
            uint32_t low;
            if (json[index++] != '\\' || json[index++] != 'u' || !tachyon_window_hex(json, &index, &low) || low < 0xDC00 || low > 0xDFFF) return 0;
            code = 0x10000 + ((code - 0xD800) << 10) + low - 0xDC00;
          } else if (code >= 0xDC00 && code <= 0xDFFF) return 0;
          break;
        default: return 0;
      }
    } else if (code >= 128) {
      int remaining; uint32_t minimum;
      if (code >= 0xC2 && code <= 0xDF) { remaining = 1; minimum = 0x80; code &= 0x1F; }
      else if (code >= 0xE0 && code <= 0xEF) { remaining = 2; minimum = 0x800; code &= 0x0F; }
      else if (code >= 0xF0 && code <= 0xF4) { remaining = 3; minimum = 0x10000; code &= 7; }
      else return 0;
      for (int byte = 0; byte < remaining; byte++) {
        unsigned char next = (unsigned char)json[index++];
        if ((next & 0xC0) != 0x80) return 0;
        code = (code << 6) | (next & 63);
      }
      if (code < minimum || code > 0x10FFFF || (code >= 0xD800 && code <= 0xDFFF)) return 0;
    }
    if (code > 0xFFFF) {
      if (used + 2 >= capacity) return 0;
      code -= 0x10000; output[used++] = (uint16_t)(0xD800 + (code >> 10)); output[used++] = (uint16_t)(0xDC00 + (code & 1023));
    } else {
      if (used + 1 >= capacity) return 0;
      output[used++] = (uint16_t)code;
    }
  }
  output[used] = 0;
  index = tachyon_skip_space(json, index + 1);
  return json[index] == ',' || json[index] == '}';
}
