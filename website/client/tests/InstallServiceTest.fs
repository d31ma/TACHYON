[<Test(typeof<SiteTestSetup>)>]
type InstallServiceTest() =
    let windows = "windows"
    let unix = "unix"

    member this.testMacOSAndLinuxShowFirst() : unit =
        let install = InstallService()
        if not (install.Shows(unix)) then failwith "the macOS and Linux command shows first"
        if install.Shows(windows) then failwith "the Windows command waits for its button"

    member this.testPickShowsOnlyTheChosenSystem() : unit =
        let install = InstallService()
        install.Pick(windows)
        if not (install.Shows(windows)) then failwith "Windows shows after its button"
        if install.Shows(unix) then failwith "only one command shows at a time"
