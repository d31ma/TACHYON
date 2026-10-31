use landlock::{
    ABI, Access, AccessNet, CompatLevel, Compatible, NetPort, Ruleset, RulesetAttr, RulesetCreated,
    RulesetCreatedAttr,
};

pub(super) struct ConnectRules;

impl ConnectRules {
    const ABI: ABI = ABI::V4;

    pub(super) fn check() -> Result<(), String> {
        Self::ruleset().map(drop)
    }

    pub(super) fn restrict(ports: &[u16]) -> Result<(), String> {
        let mut ruleset = Self::ruleset()?;
        for port in ports {
            ruleset = ruleset
                .add_rule(NetPort::new(*port, AccessNet::ConnectTcp))
                .map_err(|error| error.to_string())?;
        }
        ruleset
            .restrict_self()
            .map(drop)
            .map_err(|error| error.to_string())
    }

    fn ruleset() -> Result<RulesetCreated, String> {
        Ruleset::default()
            .set_compatibility(CompatLevel::HardRequirement)
            .handle_access(AccessNet::from_all(Self::ABI))
            .and_then(Ruleset::create)
            .map_err(|error| format!("Landlock network rules are unavailable ({error})"))
    }
}
