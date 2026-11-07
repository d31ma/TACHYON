[Service]
public class DelayService
{
    public int Count = 0;

    public async System.Threading.Tasks.Task<BehaviorJSON> Load(BehaviorJSON key)
    {
        Count++;
        try
        {
            await TaskSignals.Hold(key.Text);
            TaskSignals.Reenter();
            Tac.EventPublish("settled", key.Text, true);
            return key;
        }
        finally { TaskSignals.Finished(key.Text); }
    }

    public async System.Threading.Tasks.Task Forget()
    {
        await TaskSignals.Hold("void");
        Count++;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> Failure()
    {
        return System.Threading.Tasks.Task.FromException<BehaviorJSON>(new System.Exception("private failure detail"));
    }
}
