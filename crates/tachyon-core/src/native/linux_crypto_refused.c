static void tachyon_crypto_start(const gchar *capability, const gchar *payload, JSCValue *value, WebKitScriptMessageReply *reply) {
  (void)capability;
  (void)payload;
  JSCValue *result = jsc_value_new_string(jsc_value_get_context(value), "{\"error\":\"tac.browserApis does not grant crypto.\"}");
  webkit_script_message_reply_return_value(reply, result);
  g_object_unref(result);
}
