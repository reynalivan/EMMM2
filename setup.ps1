[CmdletBinding()]
param(
    [switch]$SkipFrontend,
    [switch]$SkipRust
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSCommandPath
Set-Location $repositoryRoot

$minimumNodeMajor = 22
$maximumNodeMajor = 24
$requiredPnpmVersion = '10.24.0'

function Require-Command {
    param([Parameter(Mandatory)][string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "'$Name' was not found. Install it, restart PowerShell, then run .\setup.ps1 again."
    }
}

function Invoke-SetupCommand {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(ValueFromRemainingArguments)][string[]]$Arguments
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Setup command failed: $FilePath $($Arguments -join ' ')"
    }
}

if (-not $SkipFrontend) {
    Require-Command node
    Require-Command corepack

    $nodeMajor = [int]((& node --version).Trim().TrimStart('v').Split('.')[0])
    if ($nodeMajor -lt $minimumNodeMajor -or $nodeMajor -gt $maximumNodeMajor) {
        throw "Node.js $minimumNodeMajor through $maximumNodeMajor is supported; found $((& node --version).Trim())."
    }

    Invoke-SetupCommand corepack enable
    $pnpmVersion = (& corepack pnpm --version).Trim()
    if ($pnpmVersion -ne $requiredPnpmVersion) {
        throw "pnpm $requiredPnpmVersion is required; Corepack resolved $pnpmVersion."
    }

    Invoke-SetupCommand corepack pnpm install --frozen-lockfile --prefer-offline
}

if (-not $SkipRust) {
    Require-Command cargo
    Require-Command rustc
    Invoke-SetupCommand cargo fetch --manifest-path src-tauri/Cargo.toml
}

Write-Host 'Setup complete. Start development with: pnpm tauri dev' -ForegroundColor Green
