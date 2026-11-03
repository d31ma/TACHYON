
type Assert =
    static member private NothingThrown = "Expected an error to be thrown, but nothing was thrown."
    static member private MessageSeparator = " ==> "
    static member private ItemSeparator = ", "

    static member equals(expected: obj, actual: obj, ?message: string) : unit =
        let failure = "expected: <" + Assert.Describe(expected) + "> but was: <" + Assert.Describe(actual) + ">"
        Assert.Confirm(Assert.Same(expected, actual), failure, message)

    static member isTrue(condition: bool, ?message: string) : unit =
        Assert.Confirm(condition, "expected: <true> but was: <false>", message)

    static member throws<'T>(action: unit -> 'T, ?message: string) : exn =
        let thrown = Assert.ThrownBy(action)
        Assert.Confirm(not (isNull thrown), Assert.NothingThrown, message)
        thrown

    static member private ThrownBy<'T>(action: unit -> 'T) : exn =
        try
            Assert.Finish(box (action ()))
            null
        with error ->
            error

    static member private Finish(result: obj) : unit =
        match result with
        | :? System.Threading.Tasks.Task as pending -> pending.GetAwaiter().GetResult()
        | _ -> ()

    static member private Same(expected: obj, actual: obj) : bool =
        if Assert.IsWhole(expected) && Assert.IsWhole(actual) then
            System.Convert.ToDecimal(expected) = System.Convert.ToDecimal(actual)
        elif Assert.IsList(expected) && Assert.IsList(actual) then
            System.Linq.Enumerable.SequenceEqual(Assert.Items(expected), Assert.Items(actual), Assert.ValueComparer())
        else
            System.Object.Equals(expected, actual)

    static member private ValueComparer() : System.Collections.Generic.IEqualityComparer<obj> =
        { new System.Collections.Generic.IEqualityComparer<obj> with
            member _.Equals(expected: obj, actual: obj) : bool = Assert.Same(expected, actual)
            member _.GetHashCode(_value: obj) : int = 0 }

    static member private IsWhole(value: obj) : bool =
        match value with
        | :? sbyte | :? byte | :? int16 | :? uint16 | :? int | :? uint32 | :? int64 | :? uint64 -> true
        | _ -> false

    static member private IsList(value: obj) : bool =
        (value :? System.Collections.IEnumerable) && not (value :? string)

    static member private Items(list: obj) : seq<obj> =
        Seq.cast<obj> (list :?> System.Collections.IEnumerable)

    static member private Describe(value: obj) : string =
        if Assert.IsList(value) then
            "[" + System.String.Join(Assert.ItemSeparator, Seq.map Assert.Describe (Assert.Items(value))) + "]"
        elif isNull value then
            "null"
        else
            value.ToString()

    static member private Confirm(passed: bool, failure: string, message: string option) : unit =
        if not passed then
            raise (System.Exception(Assert.Text(failure, message)))

    static member private Text(failure: string, message: string option) : string =
        match message with
        | Some text -> text + Assert.MessageSeparator + failure
        | None -> failure
