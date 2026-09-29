[<Test(typeof<SiteTestSetup>)>]
type InstallServiceTest() =
    let windows = "windows"
    let unix = "unix"

    member this.testMacOSAndLinuxShowFirst() : unit =
        let install = InstallService()
        Assert.isTrue(install.Shows(unix), "the macOS and Linux command shows first")
        Assert.equals(false, install.Shows(windows), "the Windows command waits for its button")

    member this.testPickShowsOnlyTheChosenSystem() : unit =
        let install = InstallService()
        install.Pick(windows)
        Assert.isTrue(install.Shows(windows), "Windows shows after its button")
        Assert.equals(false, install.Shows(unix), "only one command shows at a time")
