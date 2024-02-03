using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

using LiteConnectionString = LiteDB.ConnectionString;

public struct ConnectionStringFFI
{
    public RustSlice<byte> filename; // not null
    public bool ReadOnly;

    public readonly LiteConnectionString ToLiteConnectionString()
    {
        var connectionString = new LiteConnectionString();
        connectionString.Filename = filename.ToUtf8String()!;
        connectionString.ReadOnly = ReadOnly;
        return connectionString;
    }
}

public static class ConnectionString
{
    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_connection_string_new")]
    public static HandleErrorResult New()
    {
        try
        {
            return new HandleErrorResult(GCHandle.Alloc(new LiteConnectionString()));
        }
        catch (Exception e)
        {
            return new HandleErrorResult(e);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_connection_string_set_file_path")]
    public static void SetFilePath(GCHandle handle, RustSlice<byte> path)
    {
        var connectionString = (LiteConnectionString)handle.Target!;
        connectionString.Filename = path.ToUtf8String();
    }
}
