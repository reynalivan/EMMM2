$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or -not $IsWindows) {
    throw 'This experiment may run only on an ephemeral GitHub-hosted Windows runner'
}
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $env:GITHUB_WORKSPACE -or $workspace -ne [IO.Path]::GetFullPath($env:GITHUB_WORKSPACE)) {
    throw 'Experiment workspace does not match the checked-out repository'
}
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'This experiment requires the already-elevated hosted runner, not a request for elevation'
}
if ($env:EMMM_E2E_POLICY_PROFILE) { throw 'Experiment profile must not be inherited from another run' }
$runtimeKey = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
Write-Output "Runtime: $((Get-ItemProperty -LiteralPath $runtimeKey).pv); elevated: True"
$profile = Join-Path $env:RUNNER_TEMP ("emmm-startup-policy-" + [guid]::NewGuid().ToString('N'))
$evidence = Join-Path $workspace '.audit-output/native-startup-experiment'
$policyRoot = 'HKLM:\SOFTWARE\Policies\Microsoft\Edge\WebView2'
$policies = @(
    @{ Path = "$policyRoot\UserDataFolder"; Value = $profile },
    @{ Path = "$policyRoot\AdditionalBrowserArguments"; Value = '--remote-debugging-address=127.0.0.1 --remote-debugging-port=0' }
)
foreach ($policy in $policies) {
    if (Test-Path -LiteralPath $policy.Path) {
        $existing = Get-ItemProperty -LiteralPath $policy.Path
        foreach ($name in @('com.reynalivan.emmm.e2e', 'emmm-e2e.exe', '*')) {
            if ($existing.PSObject.Properties.Name -contains $name) {
                throw "Refusing to overwrite an existing WebView2 policy: $($policy.Path) / $name"
            }
        }
    }
}
New-Item -ItemType Directory -Path $profile | Out-Null
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
$owned = [Collections.Generic.List[object]]::new()
$testExit = $null
$cleanupFailures = [Collections.Generic.List[string]]::new()
try {
    foreach ($policy in $policies) {
        $subkey = $policy.Path.Substring('HKLM:\'.Length)
        [Microsoft.Win32.Registry]::LocalMachine.CreateSubKey($subkey).Dispose()
        New-ItemProperty -LiteralPath $policy.Path -Name 'emmm-e2e.exe' -PropertyType String -Value $policy.Value | Out-Null
        $owned.Add($policy)
    }
    $env:EMMM_E2E_POLICY_PROFILE = $profile
    Push-Location -LiteralPath $workspace
    try {
        & pnpm exec wdio run wdio.startup-policy-experiment.conf.ts
        $testExit = $LASTEXITCODE
        if ($testExit -eq 0) {
            $session = Get-Content -LiteralPath (Join-Path $evidence 'session.json') -Raw | ConvertFrom-Json
            if ($session.profile -ne $profile -or -not $session.loopbackEndpointVerified) {
                throw 'Startup experiment did not verify its own profile and loopback endpoint'
            }
        }
    } finally {
        Pop-Location
    }
} finally {
    Remove-Item Env:\EMMM_E2E_POLICY_PROFILE -ErrorAction SilentlyContinue
    foreach ($policy in $owned) {
        try {
            $current = Get-ItemPropertyValue -LiteralPath $policy.Path -Name 'emmm-e2e.exe'
            if ($current -ne $policy.Value) { throw 'Owned policy changed; refusing to remove unfamiliar state' }
            Remove-ItemProperty -LiteralPath $policy.Path -Name 'emmm-e2e.exe'
            if ((Get-ItemProperty -LiteralPath $policy.Path).PSObject.Properties.Name -contains 'emmm-e2e.exe') {
                throw 'Owned policy was not removed'
            }
        } catch {
            $cleanupFailures.Add("$($policy.Path): $($_.Exception.Message)")
        }
    }
    @{
        profile = $profile
        policyValuesCreated = $owned.Count
        cleanupVerified = $cleanupFailures.Count -eq 0
        cleanupFailures = $cleanupFailures.ToArray()
        testExitCode = $testExit
        profileRetainedUntilRunnerDisposal = $true
    } | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $evidence 'lifecycle.json')
    if ($cleanupFailures.Count -gt 0) { throw ($cleanupFailures -join '; ') }
}
if ($testExit -ne 0) { throw "Native startup policy experiment failed: $testExit" }
