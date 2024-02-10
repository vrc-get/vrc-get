using LiteDB;

namespace vrc_get_litedb;

public struct UnityVersionFFI
{
    public RustSlice<byte> Path;
    public RustSlice<byte> Version;
    public ObjectId Id;
    public byte LoadedFromHub;
    
    public UnityVersionFFI(BsonDocument document)
    {
        Path = RustSlice.NewBoxedStrOnRustMemory(document["Path"].AsString ?? throw new Exception()); // required
        var version = document["Version"];
        Version = version.IsString ? RustSlice.NewBoxedStrOnRustMemory(version.AsString) : default;
        this.Id = new ObjectId(document["_id"].AsObjectId); // required
        var loadedFromHub = document["LoadedFromHub"];
        LoadedFromHub = (byte)(loadedFromHub.IsBoolean && loadedFromHub.AsBoolean ? 1 : 0);
    }

    public readonly BsonDocument ToBsonDocument()
    {
        return new BsonDocument
        {
            ["_id"] = Id.ToLiteObjectId(),
            ["Path"] = Path.ToUtf8String(),
            ["Version"] = Version.ToUtf8String(),
            ["LoadedFromHub"] = LoadedFromHub != 0,
        };
    }
}
