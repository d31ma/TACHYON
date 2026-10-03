use super::css_token::CssToken;

pub(super) struct FoundationColors;

impl FoundationColors {
    const FUNCTIONS: [&'static str; 12] = [
        "var",
        "rgb",
        "rgba",
        "hsl",
        "hsla",
        "hwb",
        "lab",
        "lch",
        "oklab",
        "oklch",
        "color",
        "color-mix",
    ];
    const NAMES: &'static str = "aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen transparent currentcolor inherit initial unset revert revert-layer none";
    const HEX_LENGTHS: [usize; 4] = [3, 4, 6, 8];
    const MAX_DEPTH: usize = 32;

    pub(super) fn background(tokens: &[CssToken<'_>]) -> bool {
        Self::atomic(tokens) | Self::function(tokens)
    }

    fn atomic(tokens: &[CssToken<'_>]) -> bool {
        let value = tokens
            .first()
            .map_or("", |token| token.text)
            .to_ascii_lowercase();
        let named = Self::NAMES.split_whitespace().any(|name| name == value);
        let hex = Self::hex(&value);
        (tokens.len() == 1) & (named | hex)
    }

    fn hex(value: &str) -> bool {
        let Some(digits) = value.strip_prefix('#') else {
            return false;
        };
        Self::HEX_LENGTHS.contains(&digits.len())
            & digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    fn function(tokens: &[CssToken<'_>]) -> bool {
        let name = tokens.first().map_or("", |token| token.text);
        Self::FUNCTIONS.contains(&name) & Self::single_function(tokens)
    }

    fn single_function(tokens: &[CssToken<'_>]) -> bool {
        let mut position = 1;
        let mut depth = 0;
        while Self::next_token(tokens, position, &mut depth) {
            position += 1;
        }
        (position == tokens.len()) & (depth == 0) & (position > 1)
    }

    fn next_token(tokens: &[CssToken<'_>], position: usize, depth: &mut usize) -> bool {
        match (
            position,
            *depth,
            tokens.get(position).map(|token| token.text),
        ) {
            (position, 0, _) if position > 1 => false,
            (_, _, Some("(")) if *depth < Self::MAX_DEPTH => {
                *depth += 1;
                true
            }
            (_, _, Some(")")) if *depth > 0 => {
                *depth -= 1;
                true
            }
            (_, 0, _) => false,
            (_, _, None) | (_, _, Some("(")) => false,
            _ => true,
        }
    }
}
