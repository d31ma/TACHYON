[Delegate]
public sealed class GreetingAnswerDelegate : BehaviorHTTPAdapter
{
    private readonly BehaviorJSON _value;

    public GreetingAnswerDelegate(BehaviorJSON value)
    {
        _value = value;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        return System.Threading.Tasks.Task.FromResult(_value);
    }
}
