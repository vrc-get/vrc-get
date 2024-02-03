using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

using HandleType = LiteDatabase;

public class DatabaseConnection
{
    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_new")]
    public unsafe static HandleErrorResult New(ConnectionStringFFI *connectionString)
    {
        try
        {
            var connection = new LiteDatabase((*connectionString).ToLiteConnectionString());
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
