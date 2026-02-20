# Post-change verification script
# All steps must pass without warnings
# Keep in sync with verify.sh

$ErrorActionPreference = "Stop"

function Invoke-LoggedCommand {
    param(
        [string]$Command,
        [string[]]$Arguments
    )

    if ($Arguments.Count -gt 0) {
        Write-Host ($Command + " " + ($Arguments -join " "))
    } else {
        Write-Host $Command
    }

    & $Command @Arguments
}

Write-Host "Building..."
Invoke-LoggedCommand "cargo" @("build", "--workspace", "--all-targets", "--features", "std", "--quiet")

Write-Host "Testing..."
Invoke-LoggedCommand "cargo" @("test", "--workspace", "--features", "std", "--quiet")

Write-Host "Clippy..."
Invoke-LoggedCommand "cargo" @("clippy", "--workspace", "--features", "std", "--quiet", "--", "-D", "warnings")

Write-Host "Docs..."
$env:RUSTDOCFLAGS = "-D warnings"
Invoke-LoggedCommand "cargo" @("doc", "--workspace", "--features", "std", "--no-deps", "--document-private-items", "--quiet")

Write-Host "Formatting..."
Invoke-LoggedCommand "cargo" @("fmt", "--all", "--quiet")

Write-Host "Publish dry-run..."
Invoke-LoggedCommand "cargo" @("publish", "--dry-run", "--allow-dirty", "--quiet", "--workspace")

Write-Host "All checks passed!"
