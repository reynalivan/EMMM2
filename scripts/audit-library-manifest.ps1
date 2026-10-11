param(
    [Parameter(Mandatory)][string]$Root,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateSet('Inventory', 'Before', 'After', 'Verify')][string]$Phase = 'Inventory'
)

$ErrorActionPreference = 'Stop'
if (Get-Process -Name emmm -ErrorAction SilentlyContinue) {
    throw 'Close the installed/development EMMM app before auditing a protected library'
}
$auditRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
$auditOutput = [System.IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
if (-not [System.IO.Directory]::Exists($auditRoot)) { throw 'Audit root must already exist' }
if ($auditOutput.Equals($auditRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $auditOutput.StartsWith($auditRoot + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Manifest output must be outside the audited library'
}
foreach ($target in @($auditRoot, $auditOutput)) {
    $ancestor = $target
    while ($ancestor) {
        if ([System.IO.Directory]::Exists($ancestor) -and
            (([System.IO.File]::GetAttributes($ancestor) -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)) {
            throw "Audit root/output must not pass through a directory link: $ancestor"
        }
        $ancestor = [System.IO.Path]::GetDirectoryName($ancestor)
    }
}
[System.IO.Directory]::CreateDirectory($auditOutput) | Out-Null

Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Collections.Generic;
using System.Security.Cryptography;
using System.Threading;
using System.Threading.Tasks;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

public sealed class ReadOnlyManifestEntry {
    public string Path { get; set; }
    public string Kind { get; set; }
    public long Length { get; set; }
    public long CreatedUtcTicks { get; set; }
    public long WrittenUtcTicks { get; set; }
    public int Attributes { get; set; }
    public string Sha256 { get; set; }
    public string LinkTarget { get; set; }
    public string Identity { get; set; }
}

public static class ReadOnlyLibraryManifest {
    private const uint MetadataOnly = 0;
    private const uint OpenExisting = 3;
    private const uint BackupSemantics = 0x02000000;
    private const uint OpenReparsePoint = 0x00200000;
    [StructLayout(LayoutKind.Sequential)]
    private struct Information {
        public uint Attributes;
        public System.Runtime.InteropServices.ComTypes.FILETIME Created;
        public System.Runtime.InteropServices.ComTypes.FILETIME Accessed;
        public System.Runtime.InteropServices.ComTypes.FILETIME Written;
        public uint Volume, SizeHigh, SizeLow, Links, IndexHigh, IndexLow;
    }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    private static extern SafeFileHandle CreateFileW(string path, uint access, FileShare share,
        IntPtr security, uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)]
    private static extern bool GetFileInformationByHandle(SafeFileHandle handle, out Information information);
    private static long Ticks(System.Runtime.InteropServices.ComTypes.FILETIME time) {
        long value = ((long)time.dwHighDateTime << 32) | (uint)time.dwLowDateTime;
        return DateTime.FromFileTimeUtc(value).Ticks;
    }
    private static string ExtendedPath(string path) {
        if (path.StartsWith(@"\\?\", StringComparison.Ordinal)) return path;
        if (path.StartsWith(@"\\", StringComparison.Ordinal)) return @"\\?\UNC\" + path.Substring(2);
        return @"\\?\" + path;
    }
    private static void ReadInformation(ReadOnlyManifestEntry entry, SafeFileHandle handle) {
        Information information;
        if (!GetFileInformationByHandle(handle, out information))
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        if ((information.Attributes & (uint)FileAttributes.ReparsePoint) != 0)
            throw new InvalidOperationException("Protected audit refuses filesystem links");
        entry.Kind = (information.Attributes & (uint)FileAttributes.Directory) != 0 ? "directory" : "file";
        entry.Length = entry.Kind == "file" ? checked((long)(((ulong)information.SizeHigh << 32) | information.SizeLow)) : 0;
        entry.CreatedUtcTicks = Ticks(information.Created);
        entry.WrittenUtcTicks = Ticks(information.Written);
        entry.Attributes = (int)information.Attributes;
        entry.Identity = information.Volume.ToString("X8") + ":" + information.IndexHigh.ToString("X8") + information.IndexLow.ToString("X8");
    }
    private static ReadOnlyManifestEntry Capture(string root, FileSystemInfo item) {
        if ((item.Attributes & FileAttributes.ReparsePoint) != 0)
            throw new InvalidOperationException("Protected audit refuses filesystem links");
        var entry = new ReadOnlyManifestEntry { Path = System.IO.Path.GetRelativePath(root, item.FullName).Replace('\\', '/') };
        // NTFS enumeration metadata can be stale; query the entry's read-only handle.
        using (var handle = CreateFileW(ExtendedPath(item.FullName), MetadataOnly, FileShare.Read | FileShare.Write | FileShare.Delete,
            IntPtr.Zero, OpenExisting, BackupSemantics | OpenReparsePoint, IntPtr.Zero)) {
            if (handle.IsInvalid) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            ReadInformation(entry, handle);
        }
        return entry;
    }
    public static ReadOnlyManifestEntry[] Collect(string root, bool hashFiles) {
        if (System.Diagnostics.Process.GetProcessesByName("emmm").Length != 0)
            throw new InvalidOperationException("An EMMM process is running; protected audit refused");
        var entries = new List<ReadOnlyManifestEntry>();
        var pending = new Stack<DirectoryInfo>();
        var rootDirectory = new DirectoryInfo(root);
        entries.Add(Capture(root, rootDirectory));
        pending.Push(rootDirectory);
        while (pending.Count > 0) {
            var directory = pending.Pop();
            foreach (var entry in directory.EnumerateFileSystemInfos()) {
                var captured = Capture(root, entry);
                entries.Add(captured);
                if (captured.Kind == "directory") pending.Push(new DirectoryInfo(entry.FullName));
            }
        }
        entries.Sort((left, right) => StringComparer.Ordinal.Compare(left.Path, right.Path));
        foreach (var entry in entries) {
            if (entry.Kind == "link") throw new InvalidOperationException("Protected audit refuses filesystem links");
        }
        if (hashFiles) {
            long hashed = 0;
            long startedFiles = 0;
            long bytes = 0;
            const int progressInterval = 10000;
            const int readBufferBytes = 1024 * 1024;
            Parallel.ForEach(entries, new ParallelOptions { MaxDegreeOfParallelism = 2 }, entry => {
                if (entry.Kind != "file") return;
                if (Interlocked.Increment(ref startedFiles) % 1000 == 0 &&
                    System.Diagnostics.Process.GetProcessesByName("emmm").Length != 0)
                    throw new InvalidOperationException("An EMMM process reopened during checksumming; audit aborted");
                var fullPath = System.IO.Path.Combine(root, entry.Path.Replace('/', System.IO.Path.DirectorySeparatorChar));
                using (var stream = new FileStream(fullPath, FileMode.Open, FileAccess.Read,
                    FileShare.Read, readBufferBytes, FileOptions.SequentialScan)) {
                    ReadInformation(entry, stream.SafeFileHandle);
                    entry.Sha256 = Convert.ToHexString(SHA256.HashData(stream));
                }
                var totalBytes = Interlocked.Add(ref bytes, entry.Length);
                var count = Interlocked.Increment(ref hashed);
                if (count % progressInterval == 0)
                    Console.WriteLine("checksum_progress files={0} gib={1:F2}", count, totalBytes / 1073741824.0);
            });
        }
        return entries.ToArray();
    }
}
'@

$started = [System.Diagnostics.Stopwatch]::StartNew()
$entries = [ReadOnlyLibraryManifest]::Collect($auditRoot, $Phase -in @('Before', 'After'))
if ($Phase -eq 'Verify') {
    $afterPath = Join-Path $auditOutput 'after-manifest.json'
    if (-not [System.IO.File]::Exists($afterPath)) { throw 'After checksum manifest missing' }
    $afterEntries = [System.Text.Json.JsonSerializer]::Deserialize(
        [System.IO.File]::ReadAllText($afterPath), [ReadOnlyManifestEntry[]],
        [System.Text.Json.JsonSerializerOptions]::new()
    )
    if ($entries.Length -ne $afterEntries.Length) { throw 'Library entry count changed after checksumming' }
    for ($index = 0; $index -lt $entries.Length; $index++) {
        if ($entries[$index].Path -cne $afterEntries[$index].Path) {
            throw 'Library paths changed after checksumming'
        }
        if ($entries[$index].Kind -eq 'file') {
            $entries[$index].Sha256 = $afterEntries[$index].Sha256
        }
    }
}
$manifestPath = Join-Path $auditOutput ($Phase.ToLowerInvariant() + '-manifest.json')
$json = [System.Text.Json.JsonSerializer]::Serialize(
    $entries, $entries.GetType(), [System.Text.Json.JsonSerializerOptions]::new()
)
[System.IO.File]::WriteAllText($manifestPath, $json, [System.Text.UTF8Encoding]::new($false))
$files = @($entries | Where-Object Kind -eq 'file')
$summary = [ordered]@{
    root = $auditRoot
    phase = $Phase
    metadataSource = 'WindowsHandleV2'
    files = $files.Count
    directories = @($entries | Where-Object Kind -eq 'directory').Count
    links = @($entries | Where-Object Kind -eq 'link').Count
    bytes = ($files | Measure-Object Length -Sum).Sum
    elapsedSeconds = [Math]::Round($started.Elapsed.TotalSeconds, 3)
    manifestSha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
}
if ($Phase -eq 'After') {
    $before = Join-Path $auditOutput 'before-manifest.json'
    if (-not [System.IO.File]::Exists($before)) { throw 'Before manifest missing' }
    $summary.unchanged = (Get-FileHash -LiteralPath $before -Algorithm SHA256).Hash -eq $summary.manifestSha256
}
if ($Phase -eq 'Verify') {
    $summary.stableSinceChecksum = (Get-FileHash -LiteralPath $afterPath -Algorithm SHA256).Hash -eq $summary.manifestSha256
}
$summaryPath = Join-Path $auditOutput ($Phase.ToLowerInvariant() + '-summary.json')
$summary | ConvertTo-Json | Set-Content -LiteralPath $summaryPath -Encoding utf8
$summary | ConvertTo-Json -Compress | Write-Output
if ($Phase -eq 'After' -and -not $summary.unchanged) {
    throw 'Library manifest changed; preserve evidence and do not modify/restore library files'
}
if ($Phase -eq 'Verify' -and -not $summary.stableSinceChecksum) {
    throw 'Library metadata changed during/after checksumming; preserve evidence without modifying real files'
}
