using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

using HandleType = LiteDatabase;

public class DatabaseConnection
{
    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_new")]
    public static unsafe HandleErrorResult New(ConnectionStringFFI *connectionString)
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

    interface ICollectionElementAccessor<T>
        where T : unmanaged
    {
        string CollectionName { get; }
        T FromBsonDocument(BsonDocument document);
        BsonDocument ToBsonDocument(in T element);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static unsafe LiteDbError GetAll<TAccess, T>(GCHandle handle, RustSlice<T> *result)
        where TAccess : struct, ICollectionElementAccessor<T>
        where T : unmanaged
    {
        try
        {
            var accessor = default(TAccess);
            var connection = (LiteDatabase)handle.Target!;
            var projects = connection.GetCollection(accessor.CollectionName).FindAll().ToArray();
            var slice = *result = RustSlice<T>.AllocRust((nuint)projects.Length);
            var asSpan = slice.AsSpan();
            for (var i = 0; i < projects.Length; i++)
                asSpan[i] = accessor.FromBsonDocument(projects[i]);
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static unsafe LiteDbError Update<TAccess, T>(GCHandle handle, T *project)
        where TAccess : struct, ICollectionElementAccessor<T>
        where T : unmanaged
    {
        try
        {
            var accessor = default(TAccess);
            var connection = (LiteDatabase)handle.Target!;
            connection.GetCollection(accessor.CollectionName).Update(accessor.ToBsonDocument(*project));
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static unsafe LiteDbError Insert<TAccess, T>(GCHandle handle, T *project)
        where TAccess : struct, ICollectionElementAccessor<T>
        where T : unmanaged
    {
        try
        {
            var accessor = default(TAccess);
            var connection = (LiteDatabase)handle.Target!;
            connection.GetCollection(accessor.CollectionName).Insert(accessor.ToBsonDocument(*project));
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static LiteDbError Delete<TAccess, T>(GCHandle handle, ObjectId objectId)
        where TAccess : struct, ICollectionElementAccessor<T>
        where T : unmanaged
    {
        try
        {
            var accessor = default(TAccess);
            var connection = (LiteDatabase)handle.Target!;
            connection.GetCollection(accessor.CollectionName).Delete(objectId.ToLiteObjectId());
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    struct ProjectsAccess : ICollectionElementAccessor<ProjectFFI>
    {
        public string CollectionName
        {
            [MethodImpl(MethodImplOptions.AggressiveInlining)]
            get => "projects";
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public ProjectFFI FromBsonDocument(BsonDocument document) => new(document);

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public BsonDocument ToBsonDocument(in ProjectFFI element) => element.ToBsonDocument();
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_get_projects")]
    public static unsafe LiteDbError GetProjects(GCHandle handle, RustSlice<ProjectFFI>* result) =>
        GetAll<ProjectsAccess, ProjectFFI>(handle, result);

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_update")]
    public static unsafe LiteDbError UpdateProject(GCHandle handle, ProjectFFI* project) =>
        Update<ProjectsAccess, ProjectFFI>(handle, project);

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_insert")]
    public static unsafe LiteDbError InsertProject(GCHandle handle, ProjectFFI* project) =>
        Insert<ProjectsAccess, ProjectFFI>(handle, project);

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_delete")]
    public static LiteDbError DeleteProject(GCHandle handle, ObjectId objectId) => 
        Delete<ProjectsAccess, ProjectFFI>(handle, objectId);

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_dispose")]
    public static void DisposeHandle(GCHandle handle)
    {
        var target = (HandleType)handle.Target!;
        target.Dispose();
    }
}
