using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

using HandleType = LiteDatabase;

public class DatabaseConnection
{
    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_new")]
    public static unsafe HandleErrorResult New(ConnectionStringFFI* connectionString)
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

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_get_all")]
    public static unsafe LiteDbError GetAll(
        GCHandle handle,
        /*in*/ RustSlice<byte> collectionName,
        /*out*/ RustSlice<RustSlice<byte>>* result
    )
    {
        try
        {
            // parse input
            var connection = (LiteDatabase)handle.Target!;
            var collectionNameString = collectionName.ToUtf8String();

            var projects = connection.GetCollection(collectionNameString).FindAll().ToArray();
            var bsonListSlice = *result = RustSlice<RustSlice<byte>>.AllocRust((nuint)projects.Length);
            var bsonListSpan = bsonListSlice.AsSpan();
            for (var i = 0; i < projects.Length; i++)
            {
                var serialized = BsonSerializer.Serialize(projects[i]);
                var bsonSlice = RustSlice<byte>.AllocRust((nuint)serialized.Length);
                Marshal.Copy(serialized, 0, (IntPtr)bsonSlice.Data, serialized.Length);
                bsonListSpan[i] = bsonSlice;
            }

            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_update")]
    public static LiteDbError Update(
        GCHandle handle,
        /*in*/ RustSlice<byte> collectionName,
        /*in*/ RustSlice<byte> value
    )
    {
        try
        {
            // parse input
            var connection = (LiteDatabase)handle.Target!;
            var collectionNameString = collectionName.ToUtf8String();
            var valueDocument = BsonSerializer.Deserialize(value.AsReadOnlySpan().ToArray(), true);

            connection.GetCollection(collectionNameString).Update(valueDocument);
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_insert")]
    public static LiteDbError Insert(
        GCHandle handle,
        /*in*/ RustSlice<byte> collectionName,
        /*in*/ RustSlice<byte> value
    )
    {
        try
        {
            // parse input
            var connection = (LiteDatabase)handle.Target!;
            var collectionNameString = collectionName.ToUtf8String();
            var valueDocument = BsonSerializer.Deserialize(value.AsReadOnlySpan().ToArray(), true);

            connection.GetCollection(collectionNameString).Insert(valueDocument);
            return default;
        }
        catch (Exception e)
        {
            return LiteDbError.FromException(e);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "vrc_get_litedb_database_connection_delete")]
    public static LiteDbError Delete(
        GCHandle handle, 
        /*in*/ RustSlice<byte> collectionName,
        ObjectId objectId
        )
    {
        try
        {
            // parse input
            var connection = (LiteDatabase)handle.Target!;
            var collectionNameString = collectionName.ToUtf8String();
            var objectIdValue = objectId.ToLiteObjectId();

            connection.GetCollection(collectionNameString).Delete(objectIdValue);
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