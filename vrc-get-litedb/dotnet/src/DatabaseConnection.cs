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

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_get_projects")]
    public unsafe static LiteDbError GetProjects(GCHandle handle, RustSlice<ProjectFFI> *result)
    {
        try
        {
            var connection = (LiteDatabase)handle.Target!;
            var projects = connection.GetCollection("projects").FindAll().ToArray();
            var slice = *result = RustSlice<ProjectFFI>.AllocRust((nuint)projects.Length);
            var asSpan = slice.AsSpan();
            for (var i = 0; i < projects.Length; i++)
                asSpan[i] = new ProjectFFI(projects[i]);
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_update")]
    public unsafe static LiteDbError Update(GCHandle handle, ProjectFFI *project)
    {
        try
        {
            var connection = (LiteDatabase)handle.Target!;
            connection.GetCollection("projects").Update(project->ToBsonDocument());
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_insert")]
    public unsafe static LiteDbError Insert(GCHandle handle, ProjectFFI *project)
    {
        try
        {
            var connection = (LiteDatabase)handle.Target!;
            connection.GetCollection("projects").Insert(project->ToBsonDocument());
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_dispose")]
    public static void DisposeHandle(GCHandle handle)
    {
        var target = (HandleType)handle.Target!;
        target.Dispose();
    }
}
