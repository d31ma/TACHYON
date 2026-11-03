class Validator {
    int calls = 0
    Object accept(@Check(Input.class) Object value) {
        calls++
        return value
    }
}

