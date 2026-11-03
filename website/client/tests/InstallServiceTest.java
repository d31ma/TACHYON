@Test(SiteTestSetup.class, {"partition":"tests"})
class InstallServiceTest {
    private static final String WINDOWS = "windows";
    private static final String UNIX = "unix";

    void testMacOSAndLinuxShowFirst() {
        InstallService install = new InstallService();
        Assert.isTrue(install.shows(UNIX), "the macOS and Linux command shows first");
        Assert.equals(false, install.shows(WINDOWS), "the Windows command waits for its button");
    }

    void testPickShowsOnlyTheChosenSystem() {
        InstallService install = new InstallService();
        install.pick(WINDOWS);
        Assert.isTrue(install.shows(WINDOWS), "Windows shows after its button");
        Assert.equals(false, install.shows(UNIX), "only one command shows at a time");
    }
}
