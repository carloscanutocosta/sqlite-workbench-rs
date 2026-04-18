# ============================================================
#  Backup — SQLite Workbench RS
#  Guarda: configurações, favoritos, histórico e binário release.
#  Destino: backups/<timestamp>/
# ============================================================

$ErrorActionPreference = "Stop"
$repoRoot   = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$backupsDir = Join-Path $repoRoot "backups"
$ts         = Get-Date -Format "yyyyMMdd-HHmmss"
$destDir    = Join-Path $backupsDir $ts

Set-Location $repoRoot

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  SQLite Workbench RS — BACKUP" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Destino : $destDir" -ForegroundColor Gray
Write-Host ""

New-Item -ItemType Directory -Path $destDir -Force | Out-Null

$copied = 0
$skipped = 0

function Copy-IfExists($src, $label) {
    if (Test-Path $src) {
        Copy-Item $src $destDir -Force
        Write-Host "  [OK]  $label" -ForegroundColor Green
        $script:copied++
    } else {
        Write-Host "  [--]  $label (não encontrado, ignorado)" -ForegroundColor DarkGray
        $script:skipped++
    }
}

# Ficheiros de configuração e estado
Copy-IfExists (Join-Path $repoRoot "settings.json")  "settings.json"
Copy-IfExists (Join-Path $repoRoot "favorites.json") "favorites.json"
Copy-IfExists (Join-Path $repoRoot "history.json")   "history.json"

# Binário release (se existir)
Copy-IfExists (Join-Path $repoRoot "target\release\sqlite-workbench-rs.exe") "sqlite-workbench-rs.exe (release)"

# Guardar metadados do backup
$meta = [ordered]@{
    timestamp   = $ts
    machine     = $env:COMPUTERNAME
    user        = $env:USERNAME
    repoRoot    = $repoRoot
    files_saved = $copied
    files_skip  = $skipped
}
$meta | ConvertTo-Json | Set-Content (Join-Path $destDir "backup-meta.json") -Encoding UTF8

# Limpar backups com mais de 30 dias
$cutoff = (Get-Date).AddDays(-30)
$old = Get-ChildItem $backupsDir -Directory | Where-Object { $_.LastWriteTime -lt $cutoff }
if ($old) {
    $old | Remove-Item -Recurse -Force
    Write-Host ""
    Write-Host "  Removidos $($old.Count) backup(s) com mais de 30 dias." -ForegroundColor DarkGray
}

Write-Host ""
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host "  BACKUP CONCLUÍDO — $copied ficheiro(s) guardado(s)" -ForegroundColor Green
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host ""
