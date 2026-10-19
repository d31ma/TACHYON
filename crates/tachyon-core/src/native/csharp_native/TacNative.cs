public static class TacNative
{
    [UnmanagedCallersOnly(EntryPoint = "tac_native_invoke")]
    public static IntPtr NativeInvoke(IntPtr request)
    {
        var text = Marshal.PtrToStringUTF8(request);
        return Marshal.StringToCoTaskMemUTF8(Tac.NativeInvoke(text ?? "{}"));
    }

    [UnmanagedCallersOnly(EntryPoint = "tac_native_free")]
    public static void NativeFree(IntPtr answer) => Marshal.FreeCoTaskMem(answer);

    [UnmanagedCallersOnly(EntryPoint = "tac_native_set_emit")]
    public static unsafe void NativeSetEmit(IntPtr emit) => Tac.SetEmit(emit);
}
