using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace vrc_get_litedb;

/// <summary>
/// This class contains low-level FFI utilities.
/// </summary>
public static class LowLevelFfiUtils
{
    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_lowlevel_free_gc_handle")]
    private static void FreeGcHandle(nint handle) => GCHandle.FromIntPtr(handle).Free();

    [DllImport("*", EntryPoint = "vrc_get_litedb_lowlevel_alloc")]
    internal static extern unsafe byte* Alloc(nuint size, nuint alignment);

    public static UTF8Encoding FfiUtf8 = new(false, true);
    public static UTF8Encoding NoErrorFfiUtf8 = new(false, false);
}

public readonly unsafe struct RustSlice<T> where T : unmanaged
{
    public readonly T* Data;
    public readonly nuint Length;

    public RustSlice(T* data, nuint length)
    {
        Data = data;
        Length = length;
    }

    // Caller must guarantee (nuint)sizeof(T) is multiple of alignment.
    // This function infers alignment from sizeof(T).
    public static RustSlice<T> AllocRust(nuint length)
    {
        var ptr = LowLevelFfiUtils.Alloc(length * (nuint)sizeof(T), InferAlignment());
        return new RustSlice<T>((T*)ptr, length);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static nuint InferAlignment() => (nuint)1 << BitOperations.TrailingZeroCount(sizeof(T));

    public Span<T> AsSpan() => new(Data, (int)Length);
    public ReadOnlySpan<T> AsReadOnlySpan() => new(Data, (int)Length);
}

public static class RustSlice {
    public static string ToUtf8String(this in RustSlice<byte> self) => LowLevelFfiUtils.FfiUtf8.GetString(self.AsReadOnlySpan());

    public static RustSlice<byte> NewBoxedStrOnRustMemory(string data, bool noException = false)
    {
        var utf8 = noException ? LowLevelFfiUtils.NoErrorFfiUtf8 : LowLevelFfiUtils.FfiUtf8;
        var length = (nuint)utf8.GetByteCount(data);
        var slice = RustSlice<byte>.AllocRust(length);
        utf8.GetBytes(data, slice.AsSpan());
        return slice;
    }
}

[StructLayout(LayoutKind.Sequential)]
internal static class Tests
{
    [DllImport("*", EntryPoint = "test_returns_hello_rust")]
    private static extern RustSlice<byte> TestReturnsHelloRust();

    [UnmanagedCallersOnly(EntryPoint = "test_call_returns_hello_rust")]
    public static bool CallReturnsHelloRust()
    {
        var helloWorld = TestReturnsHelloRust();
        return helloWorld.ToUtf8String() == "Hello, Rust!";
    }
    
    [UnmanagedCallersOnly(EntryPoint = "test_returns_hello_csharp")]
    public static RustSlice<byte> TestReturnsHelloCsharp() => RustSlice.NewBoxedStrOnRustMemory("Hello, C#!");

    [UnmanagedCallersOnly(EntryPoint = "test_struct_size_offset_test_cs")]
    public static unsafe bool TestStructSizeOffsetTestCs()
    {
        try
        {
            var pointerSize = sizeof(nuint);

            AssertThat(sizeof(RustSlice<byte>) == 2 * pointerSize);
            AssertThat(Marshal.OffsetOf<RustSlice<byte>>("Data") == 0 * pointerSize);
            AssertThat(Marshal.OffsetOf<RustSlice<byte>>("Length") == 1 * pointerSize);

            return true;
        }
        catch (Exception e)
        {
            Console.Error.WriteLine(e);
            return false;
        }
    }

    private static void AssertThat(bool condition, [CallerArgumentExpression("condition")] string message = "")
    {
        if (!condition)
        {
            throw new Exception(message);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "throws_exception_cs")]
    public static void ThrowsExceptionCs()
    {
        try
        {
            throw new Exception("Hello, Rust! Could you see this?");
        }
        catch (Exception e)
        {
            Console.Error.WriteLine(e);
        }
    }
}
