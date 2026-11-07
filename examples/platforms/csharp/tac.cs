using System;
using System.Runtime.InteropServices;

public class Companion
{
    public string Runtime => "C# on " + RuntimeInformation.OSDescription;

    public string Machine => Environment.MachineName;

    public string Architecture => RuntimeInformation.ProcessArchitecture.ToString();

    public int Cores() => Environment.ProcessorCount;

    public string AppData() =>
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
}
