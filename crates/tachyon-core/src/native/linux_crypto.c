#include <libsecret/secret.h>
#include <openssl/core_names.h>
#include <openssl/crypto.h>
#include <openssl/evp.h>
#include <openssl/kdf.h>
#include <openssl/rand.h>

#define TACHYON_CRYPTO_KEY_BYTES 32
#define TACHYON_CRYPTO_SIGNATURE_BYTES 64
#define TACHYON_CRYPTO_NONCE_BYTES 12
#define TACHYON_CRYPTO_TAG_BYTES 16
#define TACHYON_CRYPTO_MOST_BYTES 32768
#define TACHYON_CRYPTO_SEALED_EXTRA 29
#define TACHYON_CRYPTO_LEAST_SALT 16
#define TACHYON_CRYPTO_MOST_CODE 1024
#define TACHYON_CRYPTO_MOST_DERIVED 1024
#define TACHYON_CRYPTO_ROUNDS 600000
#define TACHYON_CRYPTO_VERSION 1
#define TACHYON_CRYPTO_MOST_NAME 64
#define TACHYON_CRYPTO_UNLOCK_SECONDS 60

typedef struct {
  gchar kind[8];
  guint8 secret[TACHYON_CRYPTO_KEY_BYTES];
  guint8 public_key[TACHYON_CRYPTO_KEY_BYTES];
  gboolean has_public;
} TachyonCryptoKey;

typedef struct {
  gchar **values;
  guint count;
} TachyonCryptoValues;

typedef gchar *(*TachyonCryptoWork)(const TachyonCryptoValues *values);

typedef struct {
  const char *capability;
  guint arguments;
  TachyonCryptoWork work;
} TachyonCryptoCall;

typedef struct {
  const TachyonCryptoCall *call;
  TachyonCryptoValues values;
  WebKitScriptMessageReply *reply;
  JSCContext *context;
  gchar *answer;
} TachyonCryptoJob;

typedef enum { TACHYON_KEY_FOUND, TACHYON_KEY_MISSING, TACHYON_KEY_LOCKED } TachyonKeyLookup;

typedef enum { TACHYON_STORE_NONE, TACHYON_STORE_LOCKED, TACHYON_STORE_OPEN } TachyonStoreState;

static GHashTable *g_crypto_keys = NULL;
static gint64 g_crypto_unlock_refused_at = 0;
static GMutex g_crypto_lock;

static const SecretSchema *tachyon_crypto_schema(void) {
  static const SecretSchema schema = {
      "dev.tachyon.CryptoKey", SECRET_SCHEMA_NONE,
      {{"application", SECRET_SCHEMA_ATTRIBUTE_STRING}, {"key", SECRET_SCHEMA_ATTRIBUTE_STRING}, {NULL, 0}},
      0, NULL, NULL, NULL, NULL, NULL, NULL, NULL};
  return &schema;
}

static void tachyon_crypto_key_free(gpointer data) {
  OPENSSL_cleanse(data, sizeof(TachyonCryptoKey));
  g_free(data);
}

static GHashTable *tachyon_crypto_keys(void) {
  if (g_crypto_keys == NULL) g_crypto_keys = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, tachyon_crypto_key_free);
  return g_crypto_keys;
}

static gchar *tachyon_crypto_reason(const char *reason) {
  return g_strdup_printf("{\"value\":{\"reason\":\"%s\"}}", reason);
}

static gchar *tachyon_crypto_value(const char *fields) {
  return g_strdup_printf("{\"value\":{%s}}", fields);
}

static gboolean tachyon_crypto_base64url(const gchar *text) {
  for (const gchar *at = text; *at != '\0'; at++) {
    if (!g_ascii_isalnum(*at) && *at != '-' && *at != '_') return FALSE;
  }
  return strlen(text) % 4 != 1;
}

static guint8 *tachyon_crypto_decode(const gchar *text, gsize most, gsize *length) {
  if (text == NULL || !tachyon_crypto_base64url(text) || strlen(text) / 4 * 3 > most + 2) return NULL;
  GString *padded = g_string_new(text);
  g_strdelimit(padded->str, "-", '+');
  g_strdelimit(padded->str, "_", '/');
  while (padded->len % 4 != 0) g_string_append_c(padded, '=');
  guint8 *bytes = padded->len == 0 ? g_malloc0(1) : g_base64_decode(padded->str, length);
  if (padded->len == 0) *length = 0;
  g_string_free(padded, TRUE);
  if (*length > most) { g_free(bytes); return NULL; }
  return bytes;
}

static guint8 *tachyon_crypto_sized(const gchar *text, gsize size) {
  gsize length = 0;
  guint8 *bytes = tachyon_crypto_decode(text, size, &length);
  if (bytes != NULL && length == size) return bytes;
  g_free(bytes);
  return NULL;
}

static gchar *tachyon_crypto_encode(const guint8 *bytes, gsize length) {
  gchar *text = g_base64_encode(bytes, length);
  g_strdelimit(text, "+", '-');
  g_strdelimit(text, "/", '_');
  gchar *padding = strchr(text, '=');
  if (padding != NULL) *padding = '\0';
  return text;
}

static gboolean tachyon_crypto_name(const gchar *name) {
  gsize length = name != NULL ? strlen(name) : 0;
  if (length == 0 || length > TACHYON_CRYPTO_MOST_NAME) return FALSE;
  for (const gchar *at = name; *at != '\0'; at++) {
    if (!g_ascii_isalnum(*at) && *at != '_' && *at != '.' && *at != '-') return FALSE;
  }
  return TRUE;
}

static gboolean tachyon_crypto_is(const gchar *value, const char *first, const char *second) {
  return g_strcmp0(value, first) == 0 || g_strcmp0(value, second) == 0;
}

static gboolean tachyon_crypto_keep(const gchar *value) {
  return g_strcmp0(value, "true") == 0;
}

static gboolean tachyon_crypto_kept_collection(SecretCollection *collection, gboolean unlocked) {
  const gchar *path = g_dbus_proxy_get_object_path(G_DBUS_PROXY(collection));
  return !g_str_has_suffix(path, "/session") && (!unlocked || !secret_collection_get_locked(collection));
}

static SecretCollection *tachyon_crypto_first_collection(GList *collections, gboolean unlocked) {
  for (GList *at = collections; at != NULL; at = at->next) {
    if (tachyon_crypto_kept_collection(at->data, unlocked)) return g_object_ref(at->data);
  }
  return NULL;
}

static SecretCollection *tachyon_crypto_listed_collection(SecretService *service) {
  if (!secret_service_load_collections_sync(service, NULL, NULL)) return NULL;
  GList *collections = secret_service_get_collections(service);
  SecretCollection *chosen = tachyon_crypto_first_collection(collections, TRUE);
  if (chosen == NULL) chosen = tachyon_crypto_first_collection(collections, FALSE);
  g_list_free_full(collections, g_object_unref);
  return chosen;
}

static SecretCollection *tachyon_crypto_collection(SecretService *service) {
  SecretCollection *collection =
      secret_collection_for_alias_sync(service, SECRET_COLLECTION_DEFAULT, SECRET_COLLECTION_NONE, NULL, NULL);
  return collection != NULL ? collection : tachyon_crypto_listed_collection(service);
}

static void tachyon_crypto_store_problem(const char *what, GError *error) {
  g_printerr("%s crypto: %s%s%s\n", TACHYON_APP_NAME, what, error != NULL ? ": " : "", error != NULL ? error->message : "");
  g_clear_error(&error);
}

static gboolean tachyon_crypto_cancel(gpointer cancellable) {
  g_cancellable_cancel(cancellable);
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_crypto_unlocked(SecretService *service, SecretCollection *collection, gboolean unlock) {
  if (!secret_collection_get_locked(collection)) return TRUE;
  gint64 now = g_get_monotonic_time();
  gboolean refused_lately = g_crypto_unlock_refused_at != 0 &&
                            now - g_crypto_unlock_refused_at < (gint64)TACHYON_CRYPTO_UNLOCK_SECONDS * G_USEC_PER_SEC;
  if (!unlock || refused_lately) return FALSE;
  GCancellable *cancellable = g_cancellable_new();
  GSource *timer = g_timeout_source_new_seconds(TACHYON_CRYPTO_UNLOCK_SECONDS);
  g_source_set_callback(timer, tachyon_crypto_cancel, g_object_ref(cancellable), g_object_unref);
  g_source_attach(timer, NULL);
  GList *objects = g_list_append(NULL, collection);
  gint unlocked = secret_service_unlock_sync(service, objects, cancellable, NULL, NULL);
  g_source_destroy(timer);
  g_source_unref(timer);
  g_list_free(objects);
  g_object_unref(cancellable);
  if (unlocked <= 0) g_crypto_unlock_refused_at = g_get_monotonic_time();
  return unlocked > 0;
}

static gchar *tachyon_crypto_open_collection(gboolean *reachable, gboolean unlock) {
  GError *error = NULL;
  SecretService *service = secret_service_get_sync(SECRET_SERVICE_NONE, NULL, &error);
  *reachable = service != NULL;
  if (service == NULL) { tachyon_crypto_store_problem("no Secret Service", error); return NULL; }
  SecretCollection *collection = tachyon_crypto_collection(service);
  gboolean open = collection != NULL && tachyon_crypto_unlocked(service, collection, unlock);
  if (!open && collection != NULL) g_printerr("%s crypto: the keyring %s is locked\n", TACHYON_APP_NAME, g_dbus_proxy_get_object_path(G_DBUS_PROXY(collection)));
  if (collection == NULL) tachyon_crypto_store_problem("no keyring collection", NULL);
  gchar *path = open ? g_strdup(g_dbus_proxy_get_object_path(G_DBUS_PROXY(collection))) : NULL;
  g_clear_object(&collection);
  g_object_unref(service);
  return path;
}

static TachyonStoreState tachyon_crypto_store_state(gboolean unlock) {
  gboolean reachable = FALSE;
  gchar *path = tachyon_crypto_open_collection(&reachable, unlock);
  TachyonStoreState state = path != NULL ? TACHYON_STORE_OPEN : reachable ? TACHYON_STORE_LOCKED : TACHYON_STORE_NONE;
  g_free(path);
  return state;
}

static gboolean tachyon_crypto_store_reachable(void) {
  SecretService *service = secret_service_get_sync(SECRET_SERVICE_NONE, NULL, NULL);
  if (service == NULL) return FALSE;
  g_object_unref(service);
  return TRUE;
}

static gboolean tachyon_crypto_store_open(void) {
  return tachyon_crypto_store_state(TRUE) == TACHYON_STORE_OPEN;
}

static gchar *tachyon_crypto_store_text(const TachyonCryptoKey *key) {
  gchar *secret = tachyon_crypto_encode(key->secret, TACHYON_CRYPTO_KEY_BYTES);
  gchar *public_key = key->has_public ? tachyon_crypto_encode(key->public_key, TACHYON_CRYPTO_KEY_BYTES) : g_strdup("");
  gchar *text = g_strdup_printf("%s:%s:%s", key->kind, secret, public_key);
  OPENSSL_cleanse(secret, strlen(secret));
  g_free(secret);
  g_free(public_key);
  return text;
}

static gboolean tachyon_crypto_parse_stored(const gchar *text, TachyonCryptoKey *key) {
  gchar **parts = g_strsplit(text, ":", 3);
  gboolean shaped = g_strv_length(parts) == 3 && strlen(parts[0]) < sizeof(key->kind);
  guint8 *secret = shaped ? tachyon_crypto_sized(parts[1], TACHYON_CRYPTO_KEY_BYTES) : NULL;
  guint8 *public_key = shaped && parts[2][0] != '\0' ? tachyon_crypto_sized(parts[2], TACHYON_CRYPTO_KEY_BYTES) : NULL;
  gboolean parsed = secret != NULL;
  if (parsed) {
    g_strlcpy(key->kind, parts[0], sizeof(key->kind));
    memcpy(key->secret, secret, TACHYON_CRYPTO_KEY_BYTES);
    key->has_public = public_key != NULL;
    if (public_key != NULL) memcpy(key->public_key, public_key, TACHYON_CRYPTO_KEY_BYTES);
    OPENSSL_cleanse(secret, TACHYON_CRYPTO_KEY_BYTES);
  }
  g_free(secret);
  g_free(public_key);
  if (g_strv_length(parts) >= 2) OPENSSL_cleanse(parts[1], strlen(parts[1]));
  g_strfreev(parts);
  return parsed;
}

static TachyonKeyLookup tachyon_crypto_stored(const gchar *name, TachyonCryptoKey **found, gboolean unlock) {
  TachyonStoreState state = tachyon_crypto_store_state(unlock);
  if (state == TACHYON_STORE_NONE) return TACHYON_KEY_MISSING;
  if (state == TACHYON_STORE_LOCKED) return TACHYON_KEY_LOCKED;
  GError *error = NULL;
  gchar *text = secret_password_lookup_sync(tachyon_crypto_schema(), NULL, &error, "application", TACHYON_BUNDLE_ID, "key", name, NULL);
  if (error != NULL) { tachyon_crypto_store_problem("the keyring refused a lookup", error); return TACHYON_KEY_LOCKED; }
  if (text == NULL) return TACHYON_KEY_MISSING;
  TachyonCryptoKey *key = g_new0(TachyonCryptoKey, 1);
  gboolean parsed = tachyon_crypto_parse_stored(text, key);
  secret_password_free(text);
  if (!parsed) { tachyon_crypto_key_free(key); return TACHYON_KEY_MISSING; }
  g_hash_table_replace(tachyon_crypto_keys(), g_strdup(name), key);
  *found = key;
  return TACHYON_KEY_FOUND;
}

static TachyonKeyLookup tachyon_crypto_lookup(const gchar *name, TachyonCryptoKey **found, gboolean unlock) {
  *found = g_hash_table_lookup(tachyon_crypto_keys(), name);
  return *found != NULL ? TACHYON_KEY_FOUND : tachyon_crypto_stored(name, found, unlock);
}

static const char *tachyon_crypto_find(const gchar *name, const char *kind, TachyonCryptoKey **found) {
  TachyonKeyLookup lookup = tachyon_crypto_lookup(name, found, TRUE);
  if (lookup == TACHYON_KEY_LOCKED) return "locked";
  if (lookup == TACHYON_KEY_MISSING) return "unknown";
  return kind == NULL || g_strcmp0((*found)->kind, kind) == 0 ? NULL : "invalid";
}

static gboolean tachyon_crypto_taken(const gchar *name, gboolean kept) {
  TachyonCryptoKey *found = NULL;
  return tachyon_crypto_lookup(name, &found, kept) == TACHYON_KEY_FOUND;
}

static const char *tachyon_crypto_save(const gchar *name, TachyonCryptoKey *key, gboolean keep) {
  if (keep) {
    gchar *text = tachyon_crypto_store_text(key);
    gchar *label = g_strdup_printf("%s key %s", TACHYON_APP_NAME, name);
    GError *error = NULL;
    gboolean reachable = FALSE;
    gchar *collection = tachyon_crypto_open_collection(&reachable, TRUE);
    gboolean stored = collection != NULL &&
                      secret_password_store_sync(tachyon_crypto_schema(), collection, label, text, NULL, &error,
                                                 "application", TACHYON_BUNDLE_ID, "key", name, NULL);
    g_free(collection);
    OPENSSL_cleanse(text, strlen(text));
    g_free(text);
    g_free(label);
    if (error != NULL) tachyon_crypto_store_problem("the keyring refused the key", error);
    if (!stored) { tachyon_crypto_key_free(key); return "locked"; }
  }
  g_hash_table_replace(tachyon_crypto_keys(), g_strdup(name), key);
  return NULL;
}

static gchar *tachyon_crypto_key_made(const gchar *name, const TachyonCryptoKey *key) {
  gchar *public_key = key->has_public ? tachyon_crypto_encode(key->public_key, TACHYON_CRYPTO_KEY_BYTES) : g_strdup("");
  gchar *fields = g_strdup_printf("\"key\":\"%s\",\"publicKey\":\"%s\"", name, public_key);
  gchar *answer = tachyon_crypto_value(fields);
  g_free(fields);
  g_free(public_key);
  return answer;
}

static gchar *tachyon_crypto_secret_saved(const gchar *name, const gchar *use, guint8 *bits, gboolean keep) {
  TachyonCryptoKey *key = g_new0(TachyonCryptoKey, 1);
  g_strlcpy(key->kind, use, sizeof(key->kind));
  memcpy(key->secret, bits, TACHYON_CRYPTO_KEY_BYTES);
  OPENSSL_cleanse(bits, TACHYON_CRYPTO_KEY_BYTES);
  const char *refused = tachyon_crypto_save(name, key, keep);
  if (refused != NULL) return tachyon_crypto_reason(refused);
  TachyonCryptoKey made = {{0}, {0}, {0}, FALSE};
  return tachyon_crypto_key_made(name, &made);
}

static gboolean tachyon_crypto_hkdf(const guint8 *input, gsize input_length, const guint8 *salt, gsize salt_length,
                                    const guint8 *info, gsize info_length, guint8 *out, gsize out_length) {
  EVP_KDF *kdf = EVP_KDF_fetch(NULL, "HKDF", NULL);
  EVP_KDF_CTX *context = kdf != NULL ? EVP_KDF_CTX_new(kdf) : NULL;
  EVP_KDF_free(kdf);
  OSSL_PARAM parameters[5];
  gsize count = 0;
  parameters[count++] = OSSL_PARAM_construct_utf8_string(OSSL_KDF_PARAM_DIGEST, (char *)"SHA256", 0);
  parameters[count++] = OSSL_PARAM_construct_octet_string(OSSL_KDF_PARAM_KEY, (void *)input, input_length);
  if (salt_length > 0) parameters[count++] = OSSL_PARAM_construct_octet_string(OSSL_KDF_PARAM_SALT, (void *)salt, salt_length);
  if (info_length > 0) parameters[count++] = OSSL_PARAM_construct_octet_string(OSSL_KDF_PARAM_INFO, (void *)info, info_length);
  parameters[count] = OSSL_PARAM_construct_end();
  gboolean derived = context != NULL && input_length > 0 && EVP_KDF_derive(context, out, out_length, parameters) == 1;
  EVP_KDF_CTX_free(context);
  return derived;
}

static gchar *tachyon_crypto_expand_into_key(const guint8 *input, const gchar *salt_text, const gchar *info_text,
                                             const TachyonCryptoValues *values, guint first) {
  const gchar *new_key = values->values[first];
  const gchar *use = values->values[first + 1];
  gsize salt_length = 0, info_length = 0;
  guint8 *salt = tachyon_crypto_decode(salt_text, TACHYON_CRYPTO_MOST_BYTES, &salt_length);
  guint8 *info = tachyon_crypto_decode(info_text, TACHYON_CRYPTO_MOST_BYTES, &info_length);
  guint8 bits[TACHYON_CRYPTO_KEY_BYTES];
  gboolean derived = salt != NULL && info != NULL &&
                     tachyon_crypto_hkdf(input, TACHYON_CRYPTO_KEY_BYTES, salt, salt_length, info, info_length, bits, sizeof(bits));
  g_free(salt);
  g_free(info);
  if (!derived) return tachyon_crypto_reason(salt == NULL || info == NULL ? "invalid" : "failed");
  return tachyon_crypto_secret_saved(new_key, use, bits, tachyon_crypto_keep(values->values[first + 2]));
}

static const char *tachyon_crypto_new_key_refusal(const TachyonCryptoValues *values, guint first) {
  if (!tachyon_crypto_name(values->values[first]) || !tachyon_crypto_is(values->values[first + 1], "encrypt", "derive")) return "invalid";
  return tachyon_crypto_taken(values->values[first], tachyon_crypto_keep(values->values[first + 2])) ? "exists" : NULL;
}

static EVP_PKEY *tachyon_crypto_private(const TachyonCryptoKey *key, int type) {
  return EVP_PKEY_new_raw_private_key(type, NULL, key->secret, TACHYON_CRYPTO_KEY_BYTES);
}

static gchar *tachyon_crypto_check(const TachyonCryptoValues *values) {
  (void)values;
  const char *store = tachyon_crypto_store_reachable() ? "os" : "none";
  gchar *fields = g_strdup_printf(
      "\"x25519\":true,\"ed25519\":true,\"aesGcm\":true,\"hkdf\":true,\"pbkdf2\":true,\"keyStore\":\"%s\"", store);
  gchar *answer = tachyon_crypto_value(fields);
  g_free(fields);
  return answer;
}

static gboolean tachyon_crypto_new_pair(const gchar *algorithm, TachyonCryptoKey *key) {
  EVP_PKEY *pair = EVP_PKEY_Q_keygen(NULL, NULL, g_strcmp0(algorithm, "x25519") == 0 ? "X25519" : "ED25519");
  size_t secret_length = TACHYON_CRYPTO_KEY_BYTES, public_length = TACHYON_CRYPTO_KEY_BYTES;
  gboolean made = pair != NULL && EVP_PKEY_get_raw_private_key(pair, key->secret, &secret_length) == 1 &&
                  EVP_PKEY_get_raw_public_key(pair, key->public_key, &public_length) == 1;
  EVP_PKEY_free(pair);
  g_strlcpy(key->kind, algorithm, sizeof(key->kind));
  key->has_public = TRUE;
  return made;
}

static gchar *tachyon_crypto_make_key_pair(const TachyonCryptoValues *values) {
  const gchar *name = values->values[0];
  if (!tachyon_crypto_name(name) || !tachyon_crypto_is(values->values[1], "x25519", "ed25519")) return tachyon_crypto_reason("invalid");
  if (tachyon_crypto_taken(name, tachyon_crypto_keep(values->values[2]))) return tachyon_crypto_reason("exists");
  TachyonCryptoKey *key = g_new0(TachyonCryptoKey, 1);
  if (!tachyon_crypto_new_pair(values->values[1], key)) { tachyon_crypto_key_free(key); return tachyon_crypto_reason("failed"); }
  TachyonCryptoKey shown = *key;
  OPENSSL_cleanse(shown.secret, TACHYON_CRYPTO_KEY_BYTES);
  const char *refused = tachyon_crypto_save(name, key, tachyon_crypto_keep(values->values[2]));
  return refused != NULL ? tachyon_crypto_reason(refused) : tachyon_crypto_key_made(name, &shown);
}

static gchar *tachyon_crypto_public_key(const TachyonCryptoValues *values) {
  TachyonCryptoKey *key = NULL;
  if (!tachyon_crypto_name(values->values[0])) return tachyon_crypto_reason("invalid");
  const char *refused = tachyon_crypto_find(values->values[0], NULL, &key);
  if (refused != NULL) return tachyon_crypto_reason(refused);
  return key->has_public ? tachyon_crypto_key_made(values->values[0], key) : tachyon_crypto_reason("invalid");
}

static gchar *tachyon_crypto_sign(const TachyonCryptoValues *values) {
  TachyonCryptoKey *key = NULL;
  gsize length = 0;
  guint8 *data = tachyon_crypto_decode(values->values[1], TACHYON_CRYPTO_MOST_BYTES, &length);
  const char *refused = tachyon_crypto_name(values->values[0]) && data != NULL
                            ? tachyon_crypto_find(values->values[0], "ed25519", &key) : "invalid";
  if (refused != NULL) { g_free(data); return tachyon_crypto_reason(refused); }
  EVP_PKEY *signer = tachyon_crypto_private(key, EVP_PKEY_ED25519);
  EVP_MD_CTX *context = EVP_MD_CTX_new();
  guint8 signature[TACHYON_CRYPTO_SIGNATURE_BYTES];
  size_t signature_length = sizeof(signature);
  gboolean signed_data = signer != NULL && context != NULL && EVP_DigestSignInit(context, NULL, NULL, NULL, signer) == 1 &&
                         EVP_DigestSign(context, signature, &signature_length, data, length) == 1;
  EVP_MD_CTX_free(context);
  EVP_PKEY_free(signer);
  g_free(data);
  if (!signed_data) return tachyon_crypto_reason("failed");
  gchar *text = tachyon_crypto_encode(signature, signature_length);
  gchar *fields = g_strdup_printf("\"signature\":\"%s\"", text);
  gchar *answer = tachyon_crypto_value(fields);
  g_free(fields);
  g_free(text);
  return answer;
}

static gboolean tachyon_crypto_verified(const guint8 *public_key, const guint8 *data, gsize length, const guint8 *signature,
                                        gsize signature_length) {
  EVP_PKEY *verifier = EVP_PKEY_new_raw_public_key(EVP_PKEY_ED25519, NULL, public_key, TACHYON_CRYPTO_KEY_BYTES);
  EVP_MD_CTX *context = EVP_MD_CTX_new();
  gboolean valid = verifier != NULL && context != NULL && signature_length == TACHYON_CRYPTO_SIGNATURE_BYTES &&
                   EVP_DigestVerifyInit(context, NULL, NULL, NULL, verifier) == 1 &&
                   EVP_DigestVerify(context, signature, signature_length, data, length) == 1;
  EVP_MD_CTX_free(context);
  EVP_PKEY_free(verifier);
  return valid;
}

static gchar *tachyon_crypto_verify(const TachyonCryptoValues *values) {
  gsize length = 0, signature_length = 0;
  guint8 *public_key = tachyon_crypto_sized(values->values[0], TACHYON_CRYPTO_KEY_BYTES);
  guint8 *data = tachyon_crypto_decode(values->values[1], TACHYON_CRYPTO_MOST_BYTES, &length);
  guint8 *signature = tachyon_crypto_decode(values->values[2], TACHYON_CRYPTO_MOST_BYTES, &signature_length);
  gboolean checked = public_key != NULL && data != NULL && signature != NULL;
  gboolean valid = checked && tachyon_crypto_verified(public_key, data, length, signature, signature_length);
  g_free(public_key);
  g_free(data);
  g_free(signature);
  if (!checked) return tachyon_crypto_reason("invalid");
  return tachyon_crypto_value(valid ? "\"valid\":true" : "\"valid\":false");
}

static gboolean tachyon_crypto_shared(const TachyonCryptoKey *key, const guint8 *peer_key, guint8 *shared) {
  EVP_PKEY *own = tachyon_crypto_private(key, EVP_PKEY_X25519);
  EVP_PKEY *peer = EVP_PKEY_new_raw_public_key(EVP_PKEY_X25519, NULL, peer_key, TACHYON_CRYPTO_KEY_BYTES);
  EVP_PKEY_CTX *context = own != NULL ? EVP_PKEY_CTX_new(own, NULL) : NULL;
  size_t length = TACHYON_CRYPTO_KEY_BYTES;
  gboolean agreed = context != NULL && peer != NULL && EVP_PKEY_derive_init(context) == 1 &&
                    EVP_PKEY_derive_set_peer(context, peer) == 1 && EVP_PKEY_derive(context, shared, &length) == 1 &&
                    length == TACHYON_CRYPTO_KEY_BYTES;
  EVP_PKEY_CTX_free(context);
  EVP_PKEY_free(peer);
  EVP_PKEY_free(own);
  guint8 any = 0;
  for (gsize index = 0; index < TACHYON_CRYPTO_KEY_BYTES; index++) any |= shared[index];
  return agreed && any != 0;
}

static gchar *tachyon_crypto_agree(const TachyonCryptoValues *values) {
  TachyonCryptoKey *key = NULL;
  guint8 *peer = tachyon_crypto_sized(values->values[1], TACHYON_CRYPTO_KEY_BYTES);
  const char *refused = tachyon_crypto_name(values->values[0]) && peer != NULL ? tachyon_crypto_new_key_refusal(values, 4) : "invalid";
  if (refused == NULL) refused = tachyon_crypto_find(values->values[0], "x25519", &key);
  if (refused != NULL) { g_free(peer); return tachyon_crypto_reason(refused); }
  guint8 shared[TACHYON_CRYPTO_KEY_BYTES] = {0};
  gboolean agreed = tachyon_crypto_shared(key, peer, shared);
  g_free(peer);
  gchar *answer = agreed ? tachyon_crypto_expand_into_key(shared, values->values[2], values->values[3], values, 4)
                         : tachyon_crypto_reason("invalid");
  OPENSSL_cleanse(shared, sizeof(shared));
  return answer;
}

static gchar *tachyon_crypto_derive_key(const TachyonCryptoValues *values) {
  TachyonCryptoKey *key = NULL;
  const char *refused = tachyon_crypto_name(values->values[0]) ? tachyon_crypto_new_key_refusal(values, 3) : "invalid";
  if (refused == NULL) refused = tachyon_crypto_find(values->values[0], "derive", &key);
  if (refused != NULL) return tachyon_crypto_reason(refused);
  return tachyon_crypto_expand_into_key(key->secret, values->values[1], values->values[2], values, 3);
}

static gchar *tachyon_crypto_import_key(const TachyonCryptoValues *values) {
  const gchar *shifted[3] = {values->values[0], values->values[2], values->values[3]};
  TachyonCryptoValues new_key = {(gchar **)shifted, 3};
  const char *refused = tachyon_crypto_new_key_refusal(&new_key, 0);
  guint8 *secret = refused == NULL ? tachyon_crypto_sized(values->values[1], TACHYON_CRYPTO_KEY_BYTES) : NULL;
  if (refused == NULL && secret == NULL) refused = "invalid";
  if (refused != NULL) return tachyon_crypto_reason(refused);
  gchar *answer = tachyon_crypto_secret_saved(values->values[0], values->values[2], secret, tachyon_crypto_keep(values->values[3]));
  g_free(secret);
  return answer;
}

static gchar *tachyon_crypto_from_code(const TachyonCryptoValues *values) {
  const gchar *shifted[3] = {values->values[0], values->values[3], values->values[4]};
  TachyonCryptoValues new_key = {(gchar **)shifted, 3};
  const gchar *code = values->values[1];
  gsize code_length = code != NULL ? strlen(code) : 0, salt_length = 0;
  guint8 *salt = tachyon_crypto_decode(values->values[2], TACHYON_CRYPTO_MOST_BYTES, &salt_length);
  const char *refused = code_length == 0 || code_length > TACHYON_CRYPTO_MOST_CODE || salt == NULL || salt_length < TACHYON_CRYPTO_LEAST_SALT
                            ? "invalid" : tachyon_crypto_new_key_refusal(&new_key, 0);
  if (refused != NULL) { g_free(salt); return tachyon_crypto_reason(refused); }
  guint8 bits[TACHYON_CRYPTO_KEY_BYTES];
  gboolean derived = PKCS5_PBKDF2_HMAC(code, (int)code_length, salt, (int)salt_length, TACHYON_CRYPTO_ROUNDS, EVP_sha256(),
                                       TACHYON_CRYPTO_KEY_BYTES, bits) == 1;
  g_free(salt);
  if (!derived) return tachyon_crypto_reason("failed");
  return tachyon_crypto_secret_saved(values->values[0], values->values[3], bits, tachyon_crypto_keep(values->values[4]));
}

static gboolean tachyon_crypto_gcm(gboolean encrypting, const TachyonCryptoKey *key, const guint8 *nonce, const guint8 *aad,
                                   gsize aad_length, const guint8 *input, gsize length, guint8 *output, guint8 *tag) {
  EVP_CIPHER_CTX *context = EVP_CIPHER_CTX_new();
  int written = 0, finished = 0;
  gboolean ready = context != NULL && EVP_CipherInit_ex(context, EVP_aes_256_gcm(), NULL, NULL, NULL, encrypting) == 1 &&
                   EVP_CIPHER_CTX_ctrl(context, EVP_CTRL_GCM_SET_IVLEN, TACHYON_CRYPTO_NONCE_BYTES, NULL) == 1 &&
                   EVP_CipherInit_ex(context, NULL, NULL, key->secret, nonce, encrypting) == 1 &&
                   (aad_length == 0 || EVP_CipherUpdate(context, NULL, &written, aad, (int)aad_length) == 1) &&
                   EVP_CipherUpdate(context, output, &written, input, (int)length) == 1;
  if (ready && !encrypting) ready = EVP_CIPHER_CTX_ctrl(context, EVP_CTRL_GCM_SET_TAG, TACHYON_CRYPTO_TAG_BYTES, tag) == 1;
  ready = ready && EVP_CipherFinal_ex(context, output + written, &finished) == 1;
  if (ready && encrypting) ready = EVP_CIPHER_CTX_ctrl(context, EVP_CTRL_GCM_GET_TAG, TACHYON_CRYPTO_TAG_BYTES, tag) == 1;
  EVP_CIPHER_CTX_free(context);
  return ready;
}

static gchar *tachyon_crypto_encrypt(const TachyonCryptoValues *values) {
  TachyonCryptoKey *key = NULL;
  gsize length = 0, aad_length = 0;
  guint8 *data = tachyon_crypto_decode(values->values[1], TACHYON_CRYPTO_MOST_BYTES, &length);
  guint8 *aad = tachyon_crypto_decode(values->values[2], TACHYON_CRYPTO_MOST_BYTES, &aad_length);
  const char *refused = tachyon_crypto_name(values->values[0]) && data != NULL && aad != NULL
                            ? tachyon_crypto_find(values->values[0], "encrypt", &key) : "invalid";
  guint8 *sealed = g_malloc0(length + TACHYON_CRYPTO_SEALED_EXTRA);
  sealed[0] = TACHYON_CRYPTO_VERSION;
  gboolean done = refused == NULL && RAND_bytes(sealed + 1, TACHYON_CRYPTO_NONCE_BYTES) == 1 &&
                  tachyon_crypto_gcm(TRUE, key, sealed + 1, aad, aad_length, data, length, sealed + 1 + TACHYON_CRYPTO_NONCE_BYTES,
                                     sealed + 1 + TACHYON_CRYPTO_NONCE_BYTES + length);
  g_free(data);
  g_free(aad);
  gchar *text = done ? tachyon_crypto_encode(sealed, length + TACHYON_CRYPTO_SEALED_EXTRA) : NULL;
  g_free(sealed);
  if (text == NULL) return tachyon_crypto_reason(refused != NULL ? refused : "failed");
  gchar *fields = g_strdup_printf("\"sealed\":\"%s\"", text);
  gchar *answer = tachyon_crypto_value(fields);
  g_free(fields);
  g_free(text);
  return answer;
}

static gchar *tachyon_crypto_opened(const TachyonCryptoKey *key, const guint8 *sealed, gsize sealed_length, const guint8 *aad,
                                    gsize aad_length) {
  if (sealed_length < TACHYON_CRYPTO_SEALED_EXTRA || sealed[0] != TACHYON_CRYPTO_VERSION) return tachyon_crypto_reason("rejected");
  gsize length = sealed_length - TACHYON_CRYPTO_SEALED_EXTRA;
  guint8 *data = g_malloc0(length + 1);
  guint8 tag[TACHYON_CRYPTO_TAG_BYTES];
  memcpy(tag, sealed + sealed_length - TACHYON_CRYPTO_TAG_BYTES, TACHYON_CRYPTO_TAG_BYTES);
  gboolean opened = tachyon_crypto_gcm(FALSE, key, sealed + 1, aad, aad_length, sealed + 1 + TACHYON_CRYPTO_NONCE_BYTES, length, data, tag);
  gchar *text = opened ? tachyon_crypto_encode(data, length) : NULL;
  OPENSSL_cleanse(data, length);
  g_free(data);
  if (text == NULL) return tachyon_crypto_reason("rejected");
  gchar *fields = g_strdup_printf("\"data\":\"%s\"", text);
  gchar *answer = tachyon_crypto_value(fields);
  g_free(fields);
  g_free(text);
  return answer;
}

static gchar *tachyon_crypto_decrypt(const TachyonCryptoValues *values) {
  TachyonCryptoKey *key = NULL;
  gsize sealed_length = 0, aad_length = 0;
  guint8 *sealed = tachyon_crypto_decode(values->values[1], TACHYON_CRYPTO_MOST_BYTES + TACHYON_CRYPTO_SEALED_EXTRA, &sealed_length);
  guint8 *aad = tachyon_crypto_decode(values->values[2], TACHYON_CRYPTO_MOST_BYTES, &aad_length);
  const char *refused = tachyon_crypto_name(values->values[0]) && sealed != NULL && aad != NULL
                            ? tachyon_crypto_find(values->values[0], "encrypt", &key) : "invalid";
  gchar *answer = refused != NULL ? tachyon_crypto_reason(refused) : tachyon_crypto_opened(key, sealed, sealed_length, aad, aad_length);
  g_free(sealed);
  g_free(aad);
  return answer;
}

static gchar *tachyon_crypto_expand(const TachyonCryptoValues *values) {
  gsize input_length = 0, salt_length = 0, info_length = 0;
  guint8 *input = tachyon_crypto_decode(values->values[0], TACHYON_CRYPTO_MOST_BYTES, &input_length);
  guint8 *salt = tachyon_crypto_decode(values->values[1], TACHYON_CRYPTO_MOST_BYTES, &salt_length);
  guint8 *info = tachyon_crypto_decode(values->values[2], TACHYON_CRYPTO_MOST_BYTES, &info_length);
  gint64 length = values->values[3] != NULL ? g_ascii_strtoll(values->values[3], NULL, 10) : 0;
  gboolean checked = input != NULL && salt != NULL && info != NULL && length >= 1 && length <= TACHYON_CRYPTO_MOST_DERIVED;
  guint8 *bytes = checked ? g_malloc0((gsize)length) : NULL;
  gboolean derived = checked && tachyon_crypto_hkdf(input, input_length, salt, salt_length, info, info_length, bytes, (gsize)length);
  gchar *text = derived ? tachyon_crypto_encode(bytes, (gsize)length) : NULL;
  g_free(input);
  g_free(salt);
  g_free(info);
  g_free(bytes);
  if (text == NULL) return tachyon_crypto_reason(checked ? "failed" : "invalid");
  gchar *fields = g_strdup_printf("\"bytes\":\"%s\"", text);
  gchar *answer = tachyon_crypto_value(fields);
  g_free(fields);
  g_free(text);
  return answer;
}

static gchar *tachyon_crypto_hash(const TachyonCryptoValues *values) {
  gsize length = 0;
  guint8 *data = tachyon_crypto_decode(values->values[0], TACHYON_CRYPTO_MOST_BYTES, &length);
  guint8 digest[EVP_MAX_MD_SIZE];
  unsigned int digest_length = 0;
  gboolean hashed = data != NULL && EVP_Digest(data, length, digest, &digest_length, EVP_sha256(), NULL) == 1;
  g_free(data);
  if (!hashed) return tachyon_crypto_reason("invalid");
  gchar *text = tachyon_crypto_encode(digest, digest_length);
  gchar *fields = g_strdup_printf("\"digest\":\"%s\"", text);
  gchar *answer = tachyon_crypto_value(fields);
  g_free(fields);
  g_free(text);
  return answer;
}

static gchar *tachyon_crypto_forget(const TachyonCryptoValues *values) {
  if (!tachyon_crypto_name(values->values[0])) return tachyon_crypto_reason("invalid");
  g_hash_table_remove(tachyon_crypto_keys(), values->values[0]);
  if (tachyon_crypto_store_open()) {
    secret_password_clear_sync(tachyon_crypto_schema(), NULL, NULL, "application", TACHYON_BUNDLE_ID, "key", values->values[0], NULL);
  }
  return tachyon_crypto_value("");
}

static gchar *tachyon_crypto_forget_all(const TachyonCryptoValues *values) {
  (void)values;
  g_hash_table_remove_all(tachyon_crypto_keys());
  if (tachyon_crypto_store_open()) {
    secret_password_clear_sync(tachyon_crypto_schema(), NULL, NULL, "application", TACHYON_BUNDLE_ID, NULL);
  }
  return tachyon_crypto_value("");
}

static const TachyonCryptoCall TACHYON_CRYPTO_CALLS[] = {
    {"browserApi.crypto.check", 0, tachyon_crypto_check},
    {"browserApi.crypto.makeKeyPair", 3, tachyon_crypto_make_key_pair},
    {"browserApi.crypto.publicKey", 1, tachyon_crypto_public_key},
    {"browserApi.crypto.sign", 2, tachyon_crypto_sign},
    {"browserApi.crypto.verify", 3, tachyon_crypto_verify},
    {"browserApi.crypto.agree", 7, tachyon_crypto_agree},
    {"browserApi.crypto.deriveKey", 6, tachyon_crypto_derive_key},
    {"browserApi.crypto.importKey", 4, tachyon_crypto_import_key},
    {"browserApi.crypto.fromCode", 5, tachyon_crypto_from_code},
    {"browserApi.crypto.encrypt", 3, tachyon_crypto_encrypt},
    {"browserApi.crypto.decrypt", 3, tachyon_crypto_decrypt},
    {"browserApi.crypto.hkdf", 4, tachyon_crypto_expand},
    {"browserApi.crypto.hash", 1, tachyon_crypto_hash},
    {"browserApi.crypto.forget", 1, tachyon_crypto_forget},
    {"browserApi.crypto.forgetAll", 0, tachyon_crypto_forget_all},
};

static const TachyonCryptoCall *tachyon_crypto_call(const gchar *capability) {
  for (gsize index = 0; index < G_N_ELEMENTS(TACHYON_CRYPTO_CALLS); index++) {
    if (g_strcmp0(TACHYON_CRYPTO_CALLS[index].capability, capability) == 0) return &TACHYON_CRYPTO_CALLS[index];
  }
  return NULL;
}

static gboolean tachyon_crypto_values(const gchar *payload, guint expected, TachyonCryptoValues *values) {
  JSCContext *context = jsc_context_new();
  JSCValue *array = jsc_value_new_from_json(context, payload);
  JSCValue *length = array != NULL && jsc_value_is_array(array) ? jsc_value_object_get_property(array, "length") : NULL;
  guint count = length != NULL ? (guint)jsc_value_to_int32(length) : G_MAXUINT;
  gboolean shaped = count == expected;
  values->values = g_new0(gchar *, expected + 1);
  values->count = expected;
  for (guint index = 0; shaped && index < expected; index++) {
    JSCValue *item = jsc_value_object_get_property_at_index(array, index);
    values->values[index] = jsc_value_is_string(item) || jsc_value_is_boolean(item) || jsc_value_is_number(item) ? jsc_value_to_string(item) : NULL;
    shaped = values->values[index] != NULL;
    g_object_unref(item);
  }
  g_clear_object(&length);
  g_clear_object(&array);
  g_object_unref(context);
  return shaped;
}

static void tachyon_crypto_values_free(TachyonCryptoValues *values) {
  for (guint index = 0; index < values->count; index++) {
    if (values->values[index] != NULL) OPENSSL_cleanse(values->values[index], strlen(values->values[index]));
  }
  g_strfreev(values->values);
}

static void tachyon_crypto_job_run(GTask *task, gpointer source, gpointer data, GCancellable *cancellable) {
  (void)source;
  (void)cancellable;
  TachyonCryptoJob *job = data;
  g_mutex_lock(&g_crypto_lock);
  job->answer = job->call->work(&job->values);
  g_mutex_unlock(&g_crypto_lock);
  g_task_return_boolean(task, TRUE);
}

static void tachyon_crypto_job_free(TachyonCryptoJob *job) {
  tachyon_crypto_values_free(&job->values);
  webkit_script_message_reply_unref(job->reply);
  g_object_unref(job->context);
  g_free(job->answer);
  g_free(job);
}

static void tachyon_crypto_job_done(GObject *source, GAsyncResult *result, gpointer data) {
  (void)source;
  (void)result;
  TachyonCryptoJob *job = data;
  JSCValue *answer = jsc_value_new_string(job->context, job->answer);
  webkit_script_message_reply_return_value(job->reply, answer);
  g_object_unref(answer);
  tachyon_crypto_job_free(job);
}

static void tachyon_crypto_answer_now(JSCValue *value, WebKitScriptMessageReply *reply, const char *answer) {
  JSCValue *result = jsc_value_new_string(jsc_value_get_context(value), answer);
  webkit_script_message_reply_return_value(reply, result);
  g_object_unref(result);
}

static void tachyon_crypto_start(const gchar *capability, const gchar *payload, JSCValue *value, WebKitScriptMessageReply *reply) {
  const TachyonCryptoCall *call = tachyon_browser_api_granted("crypto") ? tachyon_crypto_call(capability) : NULL;
  if (call == NULL) { tachyon_crypto_answer_now(value, reply, "{\"error\":\"tac.browserApis does not grant crypto.\"}"); return; }
  TachyonCryptoJob *job = g_new0(TachyonCryptoJob, 1);
  if (!tachyon_crypto_values(payload, call->arguments, &job->values)) {
    tachyon_crypto_values_free(&job->values);
    g_free(job);
    tachyon_crypto_answer_now(value, reply, "{\"value\":{\"reason\":\"invalid\"}}");
    return;
  }
  job->call = call;
  job->reply = webkit_script_message_reply_ref(reply);
  job->context = g_object_ref(jsc_value_get_context(value));
  GTask *task = g_task_new(NULL, NULL, tachyon_crypto_job_done, job);
  g_task_set_task_data(task, job, NULL);
  g_task_run_in_thread(task, tachyon_crypto_job_run);
  g_object_unref(task);
}
