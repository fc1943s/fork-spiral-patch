#:package Mono.Cecil@0.11.6

using System.Globalization;
using Mono.Cecil;
using Mono.Cecil.Cil;

const string MarkerNamespace = "Fork";
const string MarkerName = "SpiralFontPatch";
const string PreferencesType = "Fork.UI.UserControls.Preferences.GeneralUserControl";
const string MinFontSizeField = "CodeEditorMinFontSize";
const int Patched = 0;
const int Usage = 2;
const int Running = 3;
const int MissingOriginal = 4;
const int VerificationFailed = 5;
const int Unpatchable = 6;

if (args.Length < 1)
{
    Console.Error.WriteLine("usage: patch.cs <fork-dir> [min-font-size]");
    return Usage;
}

var forkDir = Path.GetFullPath(args[0]);
var minFontSize = args.Length > 1 ? double.Parse(args[1], CultureInfo.InvariantCulture) : 1.0;
var exe = Path.Combine(forkDir, "Fork.exe");
var original = exe + ".orig";
var staged = exe + ".patched";

var running = System.Diagnostics.Process.GetProcessesByName("Fork")
    .Where(process => string.Equals(ResolvedDirectory(TryPath(process)), forkDir, StringComparison.OrdinalIgnoreCase))
    .Select(process => process.Id)
    .ToArray();
if (running.Length > 0)
{
    Console.Error.WriteLine($"Fork is running from {forkDir} (pid {string.Join(", ", running)}); close it first.");
    return Running;
}

if (!HasMarker(exe))
{
    if (MinFontSize(exe) is null)
    {
        Console.Error.WriteLine(
            $"{exe} has no readable {PreferencesType}.{MinFontSizeField} (packed or changed build).");
        return Unpatchable;
    }
    File.Copy(exe, original, overwrite: true);
    Console.WriteLine($"original saved: {original}");
}
else if (!File.Exists(original))
{
    Console.Error.WriteLine($"{exe} is patched but {original} is missing; reinstall Fork to get a clean binary.");
    return MissingOriginal;
}

var windows = Environment.GetFolderPath(Environment.SpecialFolder.Windows);
var framework = Path.Combine(windows, "Microsoft.NET", "Framework64", "v4.0.30319");
var resolver = new DefaultAssemblyResolver();
foreach (var dir in new[] { forkDir, framework, Path.Combine(framework, "WPF") })
    resolver.AddSearchDirectory(dir);
var readParameters = new ReaderParameters { AssemblyResolver = resolver, InMemory = true };
using (var module = ModuleDefinition.ReadModule(original, readParameters))
{
    var load = MinFontSizeLoad(module)
        ?? throw new InvalidOperationException($"{MinFontSizeField} is not set from a constant in {original}");
    Console.WriteLine($"{MinFontSizeField}: {load.Operand} -> {minFontSize}");
    load.Operand = minFontSize;
    module.Types.Add(new TypeDefinition(MarkerNamespace, MarkerName,
        TypeAttributes.NotPublic | TypeAttributes.Abstract | TypeAttributes.Sealed, module.TypeSystem.Object));
    module.Write(staged);
}

if (!HasMarker(staged) || MinFontSize(staged) != minFontSize)
{
    Console.Error.WriteLine($"verification failed for {staged}");
    return VerificationFailed;
}
File.Move(staged, exe, overwrite: true);
Console.WriteLine($"patched: {exe}");
return Patched;

bool HasMarker(string path)
{
    using var module = ModuleDefinition.ReadModule(path, new ReaderParameters { InMemory = true });
    return module.GetType(MarkerNamespace, MarkerName) is not null;
}

double? MinFontSize(string path)
{
    using var module = ModuleDefinition.ReadModule(path, new ReaderParameters { InMemory = true });
    return MinFontSizeLoad(module)?.Operand as double?;
}

static Instruction? MinFontSizeLoad(ModuleDefinition module)
{
    var initializer = module.GetType(PreferencesType)?.Methods.SingleOrDefault(m => m.IsConstructor && m.IsStatic);
    var store = initializer?.Body.Instructions.SingleOrDefault(i =>
        i.OpCode == OpCodes.Stsfld && i.Operand is FieldReference { Name: MinFontSizeField });
    return store?.Previous is { OpCode.Code: Code.Ldc_R8 } load ? load : null;
}

static string? ResolvedDirectory(string? path)
{
    var dir = Path.GetDirectoryName(path);
    return dir is null ? null : new DirectoryInfo(dir).ResolveLinkTarget(returnFinalTarget: true)?.FullName ?? dir;
}

static string? TryPath(System.Diagnostics.Process process)
{
    try { return process.MainModule?.FileName; }
    catch (System.ComponentModel.Win32Exception) { return null; }
}
