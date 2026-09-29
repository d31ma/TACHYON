@Service
class ReadingTimeService {
    fun label(words: Int): String = phrase(minutes(words))

    private fun minutes(words: Int): Int = maxOf(ONE_MINUTE, (words + WORDS_PER_MINUTE - 1) / WORDS_PER_MINUTE)

    private fun phrase(minutes: Int): String {
        if (minutes == ONE_MINUTE) return "About 1 minute to read"
        return "About $minutes minutes to read"
    }

    companion object {
        private const val WORDS_PER_MINUTE = 200
        private const val ONE_MINUTE = 1
    }
}
