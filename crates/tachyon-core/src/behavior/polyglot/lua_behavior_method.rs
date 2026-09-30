pub(crate) struct LuaBehaviorMethod {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) parameters: Vec<String>,
    pub(crate) validated: Option<(usize, String)>,
    pub(crate) resource: Option<&'static str>,
}
