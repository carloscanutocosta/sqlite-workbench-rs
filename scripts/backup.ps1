# ============================================================
#  Backup — SQLite Workbench RS
#  Guarda: configurações, favoritos, histórico e binário release.
#  Destino: backups/<timestamp>/
# ============================================================

param (
    [string]$Path = "D:\Backup\sqlite_workbench"
)

$ErrorActionPreference = "Stop"
$repoRoot   = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$backupsDir = $Path
$ts         = Get-Date -Format "yyyyMMdd-HHmmss"
$destDir    = Join-Path $backupsDir $ts

# Validar se o disco/caminho está acessível
$drive = Split-Path $backupsDir -Qualifier
if ($drive -and -not (Test-Path $drive)) {
    Write-Host "ERRO: O disco '$drive' não está acessível ou não existe." -ForegroundColor Red
    Write-Host "Certifica-te de que a unidade está ligada e tenta novamente." -ForegroundColor Gray
    Write-Host ""
    exit 1
}

# Garantir que a pasta base de backups existe
if (-not (Test-Path $backupsDir)) {
    New-Item -ItemType Directory -Path $backupsDir -Force | Out-Null
    Write-Host ">>> Pasta base de backup criada: $backupsDir" -ForegroundColor Gray
}

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

# Rodar log se exceder 1MB e registar a operação
$logFile = Join-Path $backupsDir "operations.log"
if (Test-Path $logFile) {
    if ((Get-Item $logFile).Length -gt 1MB) {
        $oldLog = Join-Path $backupsDir "operations.old.log"
        Move-Item -Path $logFile -Destination $oldLog -Force
    }
}

$logEntry = "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] [BACKUP] Sucesso: $copied ficheiros guardados em $ts por $env:USERNAME"
Add-Content -Path $logFile -Value $logEntry -Encoding UTF8

Write-Host ""
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host "  BACKUP CONCLUÍDO — $copied ficheiro(s) guardado(s)" -ForegroundColor Green
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host ""
