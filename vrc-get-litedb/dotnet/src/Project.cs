using LiteDB;

namespace vrc_get_litedb;



public struct ProjectFFI
{
    public RustSlice<byte> Path;
    public RustSlice<byte> UnityVersion;
    public ulong CreatedAt; // milliseconds since Unix epoch in UTC
    public ulong LastModified; // milliseconds since Unix epoch in UTC
    public int Type; // project type (enum)
    public ObjectId Id; // wether if this project is favorite or not
    public byte Favorite; // wether if this project is favorite or not

    public ProjectFFI(BsonDocument document)
    {
        Path = RustSlice.NewBoxedStrOnRustMemory(document["Path"].AsString ?? throw new Exception()); // required
        var unityVersion = document["UnityVersion"];
        UnityVersion = unityVersion.IsString ? RustSlice.NewBoxedStrOnRustMemory(unityVersion.AsString) : default;
        var createdAt = document["CreatedAt"];
        CreatedAt = createdAt.IsDateTime ? createdAt.AsDateTime.ToUnixMilliseconds() : 0;
        var lastModified = document["LastModified"];
        LastModified = lastModified.IsDateTime ? lastModified.AsDateTime.ToUnixMilliseconds() : 0;
        var type = document["Type"];
        Type = type.IsInt32 ? type.AsInt32 : 0;
        Id = new ObjectId(document["_id"].AsObjectId); // required
        var favorite = document["Favorite"];
        Favorite = (byte)(favorite.IsBoolean && favorite.AsBoolean ? 1 : 0);
    }

    public readonly BsonDocument ToBsonDocument()
    {
        return new BsonDocument
        {
            ["_id"] = Id.ToLiteObjectId(),
            ["Path"] = Path.ToUtf8String(),
            ["Type"] = Type,
            ["Favorite"] = Favorite != 0,
            ["UnityVersion"] = UnityVersion.ToUtf8String(),
            ["CreatedAt"] = CreatedAt.ToDateTimeFromUnixMilliseconds(),
            ["LastModified"] = LastModified.ToDateTimeFromUnixMilliseconds(),
        };
    }
}
