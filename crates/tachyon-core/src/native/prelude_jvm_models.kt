
private fun tacModelFields(kind: Class<*>): List<java.lang.reflect.Field> =
    kind.declaredFields.filter { !java.lang.reflect.Modifier.isStatic(it.modifiers) && !it.isSynthetic }

fun tacModel(kind: kotlin.reflect.KClass<*>, value: Any?): Any? = tacModelRead(kind.java, value)

fun tacModel(kind: Class<*>, value: Any?): Any? {
    val map = value as? Map<*, *> ?: throw IllegalArgumentException("The event payload is not a model.")
    val fields = tacModelFields(kind)
    val constructor = kind.declaredConstructors.firstOrNull { !it.isSynthetic && it.parameterCount == fields.size }
        ?: throw IllegalArgumentException("The model has no constructor that takes every field.")
    constructor.isAccessible = true
    val arguments = Array(fields.size) { index -> tacModelRead(constructor.genericParameterTypes[index], map[fields[index].name]) }
    return constructor.newInstance(*arguments)
}

private fun tacModelRead(type: java.lang.reflect.Type, value: Any?): Any? {
    if (type is java.lang.reflect.WildcardType) return tacModelRead(type.upperBounds[0], value)
    if (type is java.lang.reflect.ParameterizedType) {
        val items = value as? List<*> ?: return value
        return items.map { tacModelRead(type.actualTypeArguments[0], it) }
    }
    val kind = type as? Class<*> ?: return value
    if (value == null) return null
    return when {
        kind == Int::class.javaPrimitiveType || kind == Int::class.javaObjectType -> tacInt(value)
        kind == Long::class.javaPrimitiveType || kind == Long::class.javaObjectType -> (value as Number).toLong()
        kind == Double::class.javaPrimitiveType || kind == Double::class.javaObjectType -> tacDouble(value)
        kind == Boolean::class.javaPrimitiveType || kind == Boolean::class.javaObjectType -> tacBoolean(value)
        kind == String::class.java -> value.toString()
        kind.isEnum -> kind.enumConstants.firstOrNull { (it as Enum<*>).name == value.toString() }
        value is Map<*, *> -> tacModel(kind, value)
        else -> value
    }
}
