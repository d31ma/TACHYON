package android.content

interface SharedPreferences {
    val all: Map<String, *>
    fun edit(): Editor
    interface Editor {
        fun putInt(key: String, value: Int): Editor
        fun putLong(key: String, value: Long): Editor
        fun putBoolean(key: String, value: Boolean): Editor
        fun putFloat(key: String, value: Float): Editor
        fun putString(key: String, value: String?): Editor
        fun apply()
    }
}
