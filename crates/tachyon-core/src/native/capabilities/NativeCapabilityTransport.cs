internal static class NativeCapabilityTransport
{
    internal static void Register(IntPtr call, IntPtr release)
    {
        NativeCapabilities.Call = call == IntPtr.Zero || release == IntPtr.Zero
            ? (_, _) => "{\"status\":\"unsupported\"}"
            : (capability, payload) => Send(call, release, capability, payload);
    }

    private static unsafe string Send(IntPtr registeredCall, IntPtr registeredFree, string capability, string payload)
    {
        var call = (delegate* unmanaged<IntPtr, IntPtr, nuint*, IntPtr>)registeredCall;
        var release = (delegate* unmanaged<IntPtr, nuint, void>)registeredFree;
        var operation = IntPtr.Zero;
        var request = IntPtr.Zero;
        var answer = IntPtr.Zero;
        nuint size = 0;
        try
        {
            operation = System.Runtime.InteropServices.Marshal.StringToCoTaskMemUTF8(capability);
            request = System.Runtime.InteropServices.Marshal.StringToCoTaskMemUTF8(payload);
            answer = call(operation, request, &size);
            if (answer == IntPtr.Zero || size == 0 || size > 1048576) return "{\"status\":\"unavailable\"}";
            return new System.Text.UTF8Encoding(false, true).GetString(new ReadOnlySpan<byte>((void*)answer, (int)size));
        }
        finally
        {
            if (answer != IntPtr.Zero) release(answer, size);
            System.Runtime.InteropServices.Marshal.FreeCoTaskMem(operation);
            System.Runtime.InteropServices.Marshal.FreeCoTaskMem(request);
        }
    }
}
