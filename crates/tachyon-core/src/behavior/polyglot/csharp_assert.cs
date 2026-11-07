
static class Assert
{
    const string NothingThrown = "Expected an error to be thrown, but nothing was thrown.";
    const string MessageSeparator = " ==> ";
    const string ItemSeparator = ", ";

    public static void equals(object expected, object actual, string message = null)
    {
        string failure = "expected: <" + Describe(expected) + "> but was: <" + Describe(actual) + ">";
        Confirm(Same(expected, actual), failure, message);
    }

    public static void isTrue(bool condition, string message = null)
    {
        Confirm(condition, "expected: <true> but was: <false>", message);
    }

    public static System.Exception throws(System.Action action, string message = null)
    {
        System.Exception thrown = ThrownBy(action);
        Confirm(thrown != null, NothingThrown, message);
        return thrown;
    }

    public static System.Exception throws(System.Func<System.Threading.Tasks.Task> action, string message = null)
    {
        return throws(() => action().GetAwaiter().GetResult(), message);
    }

    static System.Exception ThrownBy(System.Action action)
    {
        try
        {
            action();
            return null;
        }
        catch (System.Exception error)
        {
            return error;
        }
    }

    static bool Same(object expected, object actual)
    {
        if (IsWhole(expected) && IsWhole(actual))
        {
            return System.Convert.ToDecimal(expected) == System.Convert.ToDecimal(actual);
        }
        if (IsList(expected) && IsList(actual))
        {
            return System.Linq.Enumerable.SequenceEqual(Items(expected), Items(actual), new ValueComparer());
        }
        return object.Equals(expected, actual);
    }

    static bool IsWhole(object value)
    {
        return value is sbyte or byte or short or ushort or int or uint or long or ulong;
    }

    static bool IsList(object value)
    {
        return value is System.Collections.IEnumerable && value is not string;
    }

    static System.Collections.Generic.IEnumerable<object> Items(object list)
    {
        return System.Linq.Enumerable.Cast<object>((System.Collections.IEnumerable)list);
    }

    static string Describe(object value)
    {
        if (IsList(value))
        {
            return "[" + string.Join(ItemSeparator, System.Linq.Enumerable.Select(Items(value), Describe)) + "]";
        }
        if (value == null)
        {
            return "null";
        }
        return value.ToString();
    }

    static void Confirm(bool passed, string failure, string message)
    {
        if (!passed)
        {
            throw new System.Exception(Text(failure, message));
        }
    }

    static string Text(string failure, string message)
    {
        if (message == null)
        {
            return failure;
        }
        return message + MessageSeparator + failure;
    }

    sealed class ValueComparer : System.Collections.Generic.IEqualityComparer<object>
    {
        public new bool Equals(object expected, object actual) => Same(expected, actual);

        public int GetHashCode(object value) => 0;
    }
}
