function Compare-Feature($name, $oldPath, $newSearchPattern) {
    Write-Host "Feature: $name"
    $oldLines = (git ls-tree -r 33b2180 $oldPath | ForEach-Object { git show "$($_.Split("`t")[1])" | Measure-Object -Line }).Lines | Measure-Object -Sum | Select-Object -ExpandProperty Sum
    if ($oldLines -eq $null) { $oldLines = 0 }
    
    $newFiles = git ls-files | Select-String $newSearchPattern
    $newLines = 0
    foreach ($f in $newFiles) {
        $lines = (Get-Content $f | Measure-Object -Line).Lines
        $newLines += $lines
    }
    Write-Host "  Old Lines ($oldPath): $oldLines"
    Write-Host "  New Lines ($newSearchPattern): $newLines"
}

Compare-Feature "collection" "src-tauri/src/services/collections" "src/modules/collections"
Compare-Feature "enable disabled" "src-tauri/src/services/mods" "src/modules/library"
Compare-Feature "filewatcher" "src-tauri/src/services/scanner/watcher" "src/modules/workspace/application/scanner/watcher"
Compare-Feature "safe unsafe (privacy)" "src-tauri/src/services/workspace_mutation/privacy" "src/modules/privacy"
Compare-Feature "auto classification" "src-tauri/src/services/objects/classification" "src/modules/catalog/application/objects/classif"
Compare-Feature "ini read" "src-tauri/src/services/mods/ini" "src/modules/library/application/mods/ini"
Compare-Feature "preview" "src-tauri/src/services/mods/preview" "src/modules/library/application/mods/preview"
Compare-Feature "objectlist (catalog)" "src-tauri/src/services/objects" "src/modules/catalog"
Compare-Feature "duplicate scanner" "src-tauri/src/services/scanner/dedup" "src/modules/workspace/application/scanner/dedup"
Compare-Feature "browser" "src-tauri/src/services/browser" "src/modules/browser"
