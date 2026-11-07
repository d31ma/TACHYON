enum class FilePickerStatus(val wire: String) {
    AVAILABLE("available"), SELECTED("selected"), CANCELLED("cancelled"), READ("read"), RELEASED("released"),
    DENIED("denied"), UNSUPPORTED("unsupported"), UNAVAILABLE("unavailable"), INVALID("invalid"), DISPOSED("disposed"), TOO_LARGE("tooLarge")
}
