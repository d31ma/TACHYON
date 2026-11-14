pub(super) struct PageTransfer;

impl PageTransfer {
    const SHARED: &'static str = concat!(
        include_str!("../template/tac-random-id.js"),
        "\n",
        include_str!("../template/tac-sha256.js"),
        "\n",
        include_str!("../template/tac-transfer.js"),
        "\n"
    );

    pub(super) fn script_with(module: &str) -> Vec<u8> {
        let source = format!("{}{module}", Self::SHARED);
        crate::template::ShippedScript::compact(&source).into_bytes()
    }
}
