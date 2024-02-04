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
        FILE_NOT_FOUND = LiteException.LiteErrorCode.FILE_NOT_FOUND, // note: file means file in litedb storage
        DATABASE_SHUTDOWN = LiteException.LiteErrorCode.DATABASE_SHUTDOWN,
        INVALID_DATABASE = LiteException.LiteErrorCode.INVALID_DATABASE,
        FILE_SIZE_EXCEEDED = LiteException.LiteErrorCode.FILE_SIZE_EXCEEDED,
        COLLECTION_LIMIT_EXCEEDED = LiteException.LiteErrorCode.COLLECTION_LIMIT_EXCEEDED,
        INDEX_DROP_ID = LiteException.LiteErrorCode.INDEX_DROP_ID,
        INDEX_DUPLICATE_KEY = LiteException.LiteErrorCode.INDEX_DUPLICATE_KEY,
        INVALID_INDEX_KEY = LiteException.LiteErrorCode.INVALID_INDEX_KEY,
        INDEX_NOT_FOUND = LiteException.LiteErrorCode.INDEX_NOT_FOUND,
        INVALID_DBREF = LiteException.LiteErrorCode.INVALID_DBREF,
        LOCK_TIMEOUT = LiteException.LiteErrorCode.LOCK_TIMEOUT,
        INVALID_COMMAND = LiteException.LiteErrorCode.INVALID_COMMAND,
        ALREADY_EXISTS_COLLECTION_NAME = LiteException.LiteErrorCode.ALREADY_EXISTS_COLLECTION_NAME,
        ALREADY_OPEN_DATAFILE = LiteException.LiteErrorCode.ALREADY_OPEN_DATAFILE,
        INVALID_TRANSACTION_STATE = LiteException.LiteErrorCode.INVALID_TRANSACTION_STATE,
        INDEX_NAME_LIMIT_EXCEEDED = LiteException.LiteErrorCode.INDEX_NAME_LIMIT_EXCEEDED,
        INVALID_INDEX_NAME = LiteException.LiteErrorCode.INVALID_INDEX_NAME,
        INVALID_COLLECTION_NAME = LiteException.LiteErrorCode.INVALID_COLLECTION_NAME,
        TEMP_ENGINE_ALREADY_DEFINED = LiteException.LiteErrorCode.TEMP_ENGINE_ALREADY_DEFINED,
        INVALID_EXPRESSION_TYPE = LiteException.LiteErrorCode.INVALID_EXPRESSION_TYPE,
        COLLECTION_NOT_FOUND = LiteException.LiteErrorCode.COLLECTION_NOT_FOUND,
        COLLECTION_ALREADY_EXIST = LiteException.LiteErrorCode.COLLECTION_ALREADY_EXIST,
        INDEX_ALREADY_EXIST = LiteException.LiteErrorCode.INDEX_ALREADY_EXIST,
        INVALID_UPDATE_FIELD = LiteException.LiteErrorCode.INVALID_UPDATE_FIELD,
        INVALID_FORMAT = LiteException.LiteErrorCode.INVALID_FORMAT,
        DOCUMENT_MAX_DEPTH = LiteException.LiteErrorCode.DOCUMENT_MAX_DEPTH,
        INVALID_CTOR = LiteException.LiteErrorCode.INVALID_CTOR,
        UNEXPECTED_TOKEN = LiteException.LiteErrorCode.UNEXPECTED_TOKEN,
        INVALID_DATA_TYPE = LiteException.LiteErrorCode.INVALID_DATA_TYPE,
        PROPERTY_NOT_MAPPED = LiteException.LiteErrorCode.PROPERTY_NOT_MAPPED,
        INVALID_TYPED_NAME = LiteException.LiteErrorCode.INVALID_TYPED_NAME,
        PROPERTY_READ_WRITE = LiteException.LiteErrorCode.PROPERTY_READ_WRITE,
        INITIALSIZE_CRYPTO_NOT_SUPPORTED = LiteException.LiteErrorCode.INITIALSIZE_CRYPTO_NOT_SUPPORTED,
        INVALID_INITIALSIZE = LiteException.LiteErrorCode.INVALID_INITIALSIZE,
        INVALID_NULL_CHAR_STRING = LiteException.LiteErrorCode.INVALID_NULL_CHAR_STRING,
        INVALID_FREE_SPACE_PAGE = LiteException.LiteErrorCode.INVALID_FREE_SPACE_PAGE,
        DATA_TYPE_NOT_ASSIGNABLE = LiteException.LiteErrorCode.DATA_TYPE_NOT_ASSIGNABLE,
        AVOID_USE_OF_PROCESS = LiteException.LiteErrorCode.AVOID_USE_OF_PROCESS,
        NOT_ENCRYPTED = LiteException.LiteErrorCode.NOT_ENCRYPTED,
        INVALID_PASSWORD = LiteException.LiteErrorCode.INVALID_PASSWORD,
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
