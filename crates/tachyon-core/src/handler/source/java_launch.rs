pub(super) struct JavaLaunch;

impl JavaLaunch {
    pub(super) const PROGRAM: &'static str = "java";
    pub(super) const IPV4_SOCKETS: &'static str = "-Djava.net.preferIPv4Stack=true";
    pub(super) const JAR_RUN: &'static [&'static str] =
        &[Self::PROGRAM, Self::IPV4_SOCKETS, "-jar"];
}
