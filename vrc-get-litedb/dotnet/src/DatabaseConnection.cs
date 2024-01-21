using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

using HandleType = LiteDatabase;

public class DatabaseConnection
{
    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_new")]
    public static HandleErrorResult New(RustSlice<byte> path)
    {
        
        try
        {
            var pathStr = path.ToUtf8String();
            var connection = new LiteDatabase(pathStr);
            return new HandleErrorResult(GCHandle.Alloc(connection));
        }
        catch (Exception e)
        {
            return new HandleErrorResult(e);
        }
    }
    
    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_dispose")]
    public static void DisposeHandle(GCHandle handle)
    {
        var target = (HandleType)handle.Target!;
        target.Dispose();
    }
}
