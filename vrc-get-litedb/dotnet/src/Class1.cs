using System.Runtime.InteropServices;

namespace vrc_get_litedb;

public class Class1
{
    [DllImport("*")]
    internal static extern int rust_callback();

    [UnmanagedCallersOnly(EntryPoint = "add_dotnet")]
    public static int Add(int a, int b)
    {
        return a + b + rust_callback();
    }
}
