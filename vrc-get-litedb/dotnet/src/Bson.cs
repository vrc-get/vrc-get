using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

public unsafe struct ObjectId
{
    fixed byte value[12];

    public ObjectId(LiteDB.ObjectId asObjectId) => asObjectId.ToByteSpan(AsSpan());

    [UnscopedRef]
    public Span<byte> AsSpan() => MemoryMarshal.CreateSpan(ref value[0], 12);

    public LiteDB.ObjectId ToLiteObjectId() => new(AsSpan());
}

static class Extensions
{
    public static ulong ToUnixMilliseconds(this DateTime dateTime)
    {
        var utc = dateTime.ToUniversalTime();
        var ts = utc - BsonValue.UnixEpoch;
        return (ulong)(ts.Ticks / TimeSpan.TicksPerMillisecond);
    }

    public static DateTime ToDateTimeFromUnixMilliseconds(this ulong dateTime) =>
        BsonValue.UnixEpoch + TimeSpan.FromMilliseconds(dateTime);
}
