pub(super) struct JsdocSignature;

impl JsdocSignature {
    const OPEN: &'static str = "/**";
    const CLOSE: &'static str = "*/";
    const PARAMETER: &'static str = "@param";
    const RESULTS: [&'static str; 2] = ["@returns", "@return"];

    pub(super) fn before(source: &str, at: usize) -> Option<(&str, &str)> {
        let written = source[..at].trim_end();
        let block_end = written.strip_suffix(Self::CLOSE)?.len();
        let block_start = written[..block_end].rfind(Self::OPEN)?;
        let block = &written[block_start..block_end];
        let parameter = Self::tagged_type(block, &[Self::PARAMETER])?;
        let result = Self::tagged_type(block, &Self::RESULTS)?;
        Some((parameter, result))
    }

    fn tagged_type<'a>(block: &'a str, tags: &[&str]) -> Option<&'a str> {
        let at = tags
            .iter()
            .find_map(|tag| block.find(tag).map(|at| at + tag.len()))?;
        let rest = block[at..].trim_start().strip_prefix('{')?;
        let close = rest.find('}')?;
        Some(rest[..close].trim())
    }
}
