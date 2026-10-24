internal static class TacNativeStart
{
    [System.Runtime.CompilerServices.ModuleInitializer]
    internal static void Start()
    {
        TacHttpTransport.Start ??= TacNativeHttp.Start;
        TacHttpTransport.Answer ??= TacNativeHttp.Answer;
        TacHttpTransport.Cancel ??= TacNativeHttp.Cancel;
        TacSocketTransport.Send ??= Tac.SendNativeSocket;
        // __TACHYON_NATIVE_BINDINGS__
    }
}
