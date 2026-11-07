#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FormPattern {
    BooleanReturn,
    BooleanCompare,
    BooleanChoice,
    AnyInsteadOfFilter,
    ContainsInsteadOfIndex,
    ElseAfterExit,
    DoubleNegation,
    NegatedEquality,
    ExtraReturn,
    JoinedIf,
    CompoundAssignment,
    DictionaryGet,
    TotalLoop,
    IsEmpty,
    ForEach,
    BooleanAssignment,
    UnnecessaryReceiver,
}

impl FormPattern {
    const FIRST_CODE: u16 = 1340;
    const RECEIVER_CODE: u16 = 1362;
    const MESSAGES: [&'static str; 17] = [
        "This `if` only returns true or false. Return the condition itself.",
        "This compares a boolean with true or false. Use the boolean itself.",
        "This choice only picks true or false. Use the condition itself.",
        "This keeps the matching items only to check that one exists. Ask for any match instead.",
        "This finds a position only to check that the value is there. Ask if it contains the value instead.",
        "The `if` part ends with return, throw or break, so `else` is not needed. Remove `else` and keep its code.",
        "Two `not` operators cancel each other. Remove both.",
        "This negates an equality check. Use the opposite check instead.",
        "This `return` at the end of the method does nothing. Remove it.",
        "This `if` holds only another `if`. Join the two conditions in one `if`.",
        "This sets a variable from its own value. Use the short form, such as `+=`.",
        "This checks for a key before it reads the key. Read the value with a default instead.",
        "This loop only joins or adds up the items. Use the built-in function instead.",
        "This checks that the size is zero. Ask if it is empty instead.",
        "This loop uses the index only to read each item. Loop over the items instead.",
        "This `if` only sets true or false. Set the condition itself.",
        "This class member needs no explicit receiver. Use its bare name.",
    ];

    pub(crate) fn code(self) -> u16 {
        self.number()
    }

    fn number(self) -> u16 {
        if self == Self::UnnecessaryReceiver {
            return Self::RECEIVER_CODE;
        }
        Self::FIRST_CODE + self as u16
    }

    pub(crate) fn message(self) -> &'static str {
        Self::MESSAGES[self as usize]
    }
}
