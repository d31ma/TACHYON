public static class NativeCapabilityRegistration
{
    [System.Runtime.InteropServices.UnmanagedCallersOnly(EntryPoint = "tac_native_set_capability_adapter")]
    public static void Register(IntPtr call, IntPtr release) => NativeCapabilityTransport.Register(call, release);
}
