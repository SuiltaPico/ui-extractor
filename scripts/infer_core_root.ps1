# Resolve local-infer-core checkout (sibling dir, nested CI checkout, or env override).
function Get-InferCoreRoot {
    param([Parameter(Mandatory)][string]$UiExtractorRoot)

    if ($env:LOCAL_INFER_CORE_ROOT) {
        $resolved = Resolve-Path $env:LOCAL_INFER_CORE_ROOT -ErrorAction Stop
        return $resolved.Path
    }

    $nested = Join-Path $UiExtractorRoot "local-infer-core"
    if (Test-Path $nested) {
        return (Resolve-Path $nested).Path
    }

    return Join-Path (Split-Path $UiExtractorRoot -Parent) "local-infer-core"
}
