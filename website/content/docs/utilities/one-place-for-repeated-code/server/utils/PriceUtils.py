class PriceUtils:
    @staticmethod
    def label(cents: int) -> str:
        return "$" + PriceUtils._dollars(cents)

    @staticmethod
    def _dollars(cents: int) -> str:
        return f"{cents / 100:.2f}"
