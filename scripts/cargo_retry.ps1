# Shared cargo helpers for release/CI scripts (mitigate crates.io HTTP/2 flakes).
if (-not $env:CARGO_HTTP_MULTIPLEXING) {
    $env:CARGO_HTTP_MULTIPLEXING = "false"
}

function Get-ScratchDir {
    if ($env:TEMP) { return $env:TEMP }
    if ($env:TMPDIR) { return $env:TMPDIR }
    return "/tmp"
}

function Invoke-CargoWithRetry {
    param(
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$CargoArgs
    )

    $maxAttempts = 5
    for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
        & cargo @CargoArgs
        if ($LASTEXITCODE -eq 0) { return }
        if ($attempt -lt $maxAttempts) {
            $delay = [Math]::Min(30, 5 * $attempt)
            Write-Warning "cargo failed (attempt $attempt/$maxAttempts, exit $LASTEXITCODE); retrying in ${delay}s..."
            Start-Sleep -Seconds $delay
        }
    }
    exit $LASTEXITCODE
}
