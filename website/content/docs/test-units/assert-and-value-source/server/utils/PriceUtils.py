class PriceUtils:
    @staticmethod
    def with_tax(cents: int) -> int:
        PriceUtils._require_positive(cents)
        return round(cents * 1.2)

    @staticmethod
    def _require_positive(cents: int) -> None:
        if cents < 0:
            raise ValueError('A price is never negative.')
