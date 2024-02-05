using System.Runtime.InteropServices;
using LiteDB;

namespace vrc_get_litedb;

// Exceptions other than LiteException are panic
[StructLayout(LayoutKind.Sequential)]
public struct LiteDbError
{
    private RustSlice<byte> message;
    private ErrorCode code;

    enum ErrorCode : int
    {
        Success = 0,
        Panicking = int.MinValue,

        #region Generic C# errors

        NotFound = -1,
        PermissionDenied = -3,
        InvalidFilename = -4,
        OtherIO = -5,

        #endregion

        #region LiteDB

        //UNKNOWN = LiteException.LiteErrorCode.UNKNOWN,
        INVALID_DATABASE = LiteException.LiteErrorCode.INVALID_DATABASE,
        INDEX_DROP_ID = LiteException.LiteErrorCode.INDEX_DROP_ID,
        INDEX_DUPLICATE_KEY = LiteException.LiteErrorCode.INDEX_DUPLICATE_KEY,
        INVALID_INDEX_KEY = LiteException.LiteErrorCode.INVALID_INDEX_KEY,
        INDEX_NOT_FOUND = LiteException.LiteErrorCode.INDEX_NOT_FOUND,
        LOCK_TIMEOUT = LiteException.LiteErrorCode.LOCK_TIMEOUT,
        ALREADY_EXISTS_COLLECTION_NAME = LiteException.LiteErrorCode.ALREADY_EXISTS_COLLECTION_NAME,
        INVALID_TRANSACTION_STATE = LiteException.LiteErrorCode.INVALID_TRANSACTION_STATE,
        INVALID_INDEX_NAME = LiteException.LiteErrorCode.INVALID_INDEX_NAME,
        INVALID_COLLECTION_NAME = LiteException.LiteErrorCode.INVALID_COLLECTION_NAME,
        INDEX_ALREADY_EXIST = LiteException.LiteErrorCode.INDEX_ALREADY_EXIST,
        INVALID_UPDATE_FIELD = LiteException.LiteErrorCode.INVALID_UPDATE_FIELD,
        INVALID_FORMAT = LiteException.LiteErrorCode.INVALID_FORMAT,
        UNEXPECTED_TOKEN = LiteException.LiteErrorCode.UNEXPECTED_TOKEN,
        INVALID_DATA_TYPE = LiteException.LiteErrorCode.INVALID_DATA_TYPE,
        INVALID_INITIALSIZE = LiteException.LiteErrorCode.INVALID_INITIALSIZE,
        INVALID_NULL_CHAR_STRING = LiteException.LiteErrorCode.INVALID_NULL_CHAR_STRING,
        INVALID_FREE_SPACE_PAGE = LiteException.LiteErrorCode.INVALID_FREE_SPACE_PAGE,
        UNSUPPORTED = LiteException.LiteErrorCode.UNSUPPORTED,
        UNKNOWN = 400,

        #endregion
    }

    public static LiteDbError FromException(Exception e)
    {
        if (e is LiteException lite)
        {
            // it's handleable error
            return new LiteDbError
            {
                message = RustSlice.NewBoxedStrOnRustMemory(lite.Message),
                // we use zero for non-error
                code = lite.ErrorCode == LiteException.LiteErrorCode.UNKNOWN ? 
                    ErrorCode.UNKNOWN : (ErrorCode)lite.ErrorCode,
            };
        }
        else if (e is IOException)
        {
            var code = e switch
            {
                FileNotFoundException or DirectoryNotFoundException => ErrorCode.NotFound,
                UnauthorizedAccessException => ErrorCode.PermissionDenied,
                PathTooLongException => ErrorCode.InvalidFilename,
                _ => ErrorCode.OtherIO,
            };

            return new LiteDbError()
            {
                message = RustSlice.NewBoxedStrOnRustMemory(e.Message, noException: true),
                code = code,
            };
        }
        else
        {
            // it's unrecoverable error, panic in rust
            return new LiteDbError()
            {
                message = RustSlice.NewBoxedStrOnRustMemory(e.ToString(), noException: true),
                code = ErrorCode.Panicking,
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
