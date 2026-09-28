class Validator {
    var calls: Int = 0
    fun accept(@Check(Input::class) value: Any?): Any? {
        calls++
        return value
    }
}

