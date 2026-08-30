Write-Host "--- Checking deep_matcher ---"
git ls-files | Select-String "deep_matcher" | Select-Object -First 5
Write-Host "--- Checking update/metadata_sync ---"
git ls-files | Select-String "update|metadata_sync" | Select-Object -First 5
Write-Host "--- Checking config/settings ---"
git ls-files | Select-String "config" | Select-Object -First 5
