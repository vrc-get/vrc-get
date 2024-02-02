using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

// Exceptions other than LiteException are panic
[StructLayout(LayoutKind.Sequential)]
public struct LiteDbError
{
    private RustSlice<byte> message;
    private int code;

    public static LiteDbError FromException(Exception e)
    {
        if (e is LiteException lite)
        {
            // it's handleable error
            return new LiteDbError
            {
                message = RustSlice.NewBoxedStrOnRustMemory(lite.Message),
                code = (int) lite.ErrorCode,
            };
        }
        else
        {
            // it's unrecoverable error, panic in rust
            return new LiteDbError()
            {
                message = RustSlice.NewBoxedStrOnRustMemory(e.ToString(), noException: true),
                code = -1,
            };
        }
    }
}

[StructLayout(LayoutKind.Sequential)]
public struct HandleErrorResult
{
    private nint _result;
    private LiteDbError _error;

    public HandleErrorResult(GCHandle result)
    {
        _result = GCHandle.ToIntPtr(result);
        _error = default;
    }

    public HandleErrorResult(Exception exception)
    {
        _result = default;
        _error = LiteDbError.FromException(exception);
    }
}
