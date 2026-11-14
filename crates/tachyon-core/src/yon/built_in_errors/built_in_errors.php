
final class BrokenRule
{
    public function __construct(public readonly string $field, public readonly string $rule) {}

    public static function from(mixed $value): self
    {
        $fields = is_array($value) ? $value : [];
        return new self((string) ($fields['field'] ?? ''), (string) ($fields['rule'] ?? ''));
    }
}

abstract class YonBuiltInError extends \Exception
{
    public readonly string $reference;

    public function __construct(public readonly int $status, array $value, ?string $message = null)
    {
        parent::__construct($message ?? static::class);
        $this->reference = (string) ($value['reference'] ?? '');
    }
}

final class NotSignedIn extends YonBuiltInError
{
    public function __construct(array $value)
    {
        parent::__construct(401, $value);
    }
}

final class RoleMissing extends YonBuiltInError
{
    public function __construct(array $value)
    {
        parent::__construct(403, $value);
    }
}

final class IdentityCheckFailed extends YonBuiltInError
{
    public function __construct(array $value)
    {
        parent::__construct(500, $value);
    }
}

final class RequestRejected extends YonBuiltInError
{
    public readonly string $part;
    public readonly string $field;

    public function __construct(array $value)
    {
        parent::__construct(400, $value, (string) ($value['message'] ?? ''));
        $this->part = (string) ($value['part'] ?? '');
        $this->field = (string) ($value['field'] ?? '');
    }
}

final class BodyTooLarge extends YonBuiltInError
{
    public readonly int $limitBytes;

    public function __construct(array $value)
    {
        parent::__construct(413, $value);
        $this->limitBytes = (int) ($value['limitBytes'] ?? 0);
    }
}

final class CheckRejected extends YonBuiltInError
{
    /** @var list<BrokenRule> */
    public readonly array $violations;
    public readonly bool $truncated;

    public function __construct(array $value)
    {
        parent::__construct(422, $value);
        $entries = is_array($value['violations'] ?? null) ? array_values($value['violations']) : [];
        $this->violations = array_map(fn (mixed $entry): BrokenRule => BrokenRule::from($entry), $entries);
        $this->truncated = ($value['truncated'] ?? false) === true;
    }
}

final class UnexpectedError extends YonBuiltInError
{
    public function __construct(array $value)
    {
        parent::__construct(500, $value);
    }
}

function __yon_built_in_error__(mixed $value): YonBuiltInError
{
    $fields = is_array($value) ? $value : [];
    return match ((string) ($fields['type'] ?? '')) {
        'NotSignedIn' => new NotSignedIn($fields),
        'RoleMissing' => new RoleMissing($fields),
        'IdentityCheckFailed' => new IdentityCheckFailed($fields),
        'RequestRejected' => new RequestRejected($fields),
        'BodyTooLarge' => new BodyTooLarge($fields),
        'CheckRejected' => new CheckRejected($fields),
        'UnexpectedError' => new UnexpectedError($fields),
        default => throw new \TypeError('Unknown built-in error.'),
    };
}
