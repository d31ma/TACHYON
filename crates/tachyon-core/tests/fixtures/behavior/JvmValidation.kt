object ValidationChecks {
    private fun fails(path: String, value: Any?) {
        try {
            Behaviors.validate("Input", value)
            error("expected validation")
        } catch (failure: Behaviors.Failure) {
            check(failure.code == "validation" && failure.path == path)
        }
    }
    private fun validInput(): MutableMap<String, Any?> = mutableMapOf("id" to "one", "name" to "é", "price" to 5)
    private fun fieldFails(field: String, value: Any?, rule: String) {
        val input = validInput()
        input[field] = value
        fails("$field.$rule", input)
    }
    fun run() {
        checkResultIsCopied()
        checkRequiredId()
        checkName()
        checkNumbersAndSizes()
        checkMissingOptionalValues()
        checkInvalidJson()
        println("Kotlin DTO validation boundary checks passed")
    }
    private fun checkResultIsCopied() {
        val input = validInput()
        input["unknown"] = true
        val result = Behaviors.validate("Input", input)
        input["name"] = "changed"
        check(result["name"] == "é" && result["unknown"] == true)
    }
    private fun checkRequiredId() {
        fieldFails("id", null, "notNull")
        val missing = validInput()
        missing.remove("id")
        fails("id.notNull", missing)
    }
    private fun checkName() {
        fieldFails("name", null, "notBlank")
        fieldFails("name", 1, "notBlank")
        fieldFails("name", "", "notBlank")
        for (code in listOf(9, 10, 11, 12, 13, 32, 133, 160, 5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199, 8200, 8201, 8202, 8232, 8233, 8239, 8287, 12288)) {
            fieldFails("name", String(Character.toChars(code)), "notBlank")
        }
        fieldFails("name", "a", "size")
        fieldFails("name", "ééééé", "size")
        val invisible = validInput()
        invisible["name"] = "\u200b"
        Behaviors.validate("Input", invisible)
    }
    private fun checkNumbersAndSizes() {
        fieldFails("price", "5", "min")
        fieldFails("price", -1, "min")
        fieldFails("price", 11, "max")
        fieldFails("optional", 1, "size")
        fieldFails("optional", listOf(1, 2, 3), "size")
        fieldFails("optional", mapOf("a" to 1, "b" to 2, "c" to 3), "size")
    }
    private fun checkMissingOptionalValues() {
        val ignored = validInput()
        ignored["price"] = null
        ignored["optional"] = null
        Behaviors.validate("Input", ignored)
        ignored.remove("price")
        ignored.remove("optional")
        Behaviors.validate("Input", ignored)
    }
    private fun checkInvalidJson() {
        fails("$.object", emptyList<Any?>())
        fails("$.json", Any())
        val notANumber = validInput()
        notANumber["price"] = Double.NaN
        fails("$.json", notANumber)
        val oversized = validInput()
        oversized["unknown"] = "x".repeat(1048576)
        fails("$.json", oversized)
        val cycle = mutableListOf<Any?>()
        cycle.add(cycle)
        fails("$.json", cycle)
    }
}
