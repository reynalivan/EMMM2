[CmdletBinding()]
param(
    [switch]$SkipFrontend,
    [switch]$SkipRust
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSCommandPath
Set-Location $repositoryRoot

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
    if ($nodeMajor -lt 20) {
        throw "Node.js 20 or newer is required; found $((& node --version).Trim())."
    }

    Invoke-SetupCommand corepack enable
    Invoke-SetupCommand corepack pnpm install --frozen-lockfile
}

if (-not $SkipRust) {
    Require-Command cargo
    Require-Command rustc
    Invoke-SetupCommand cargo fetch --manifest-path src-tauri/Cargo.toml
}

Write-Host 'Setup complete. Start development with: pnpm tauri dev' -ForegroundColor Green
