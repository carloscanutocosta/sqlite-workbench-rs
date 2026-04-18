# ============================================================
#  Restore — SQLite Workbench RS
#  Lista os backups disponíveis e restaura o escolhido.
#  Faz backup automático do estado atual antes de restaurar.
# ============================================================

$ErrorActionPreference = "Stop"
$repoRoot   = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$backupsDir = Join-Path $repoRoot "backups"

Set-Location $repoRoot

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  SQLite Workbench RS — RESTORE" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Verificar se existem backups
if (-not (Test-Path $backupsDir)) {
    Write-Host "  Nenhum backup encontrado em: $backupsDir" -ForegroundColor Red
    exit 0
}

$backups = Get-ChildItem $backupsDir -Directory | Sort-Object LastWriteTime -Descending

if ($backups.Count -eq 0) {
    Write-Host "  Nenhum backup disponível." -ForegroundColor Red
    exit 0
}

# Listar backups disponíveis
Write-Host "  Backups disponíveis:" -ForegroundColor Yellow
Write-Host ""

$i = 1
foreach ($b in $backups) {
    $metaFile = Join-Path $b.FullName "backup-meta.json"
    $label    = $b.Name

    $files = ""
    if (Test-Path $metaFile) {
        $meta  = Get-Content $metaFile | ConvertFrom-Json
        $files = "  ($($meta.files_saved) ficheiro(s))"
    }

    $age = ((Get-Date) - $b.LastWriteTime)
    $ageStr = if ($age.Days -gt 0) { "$($age.Days)d atrás" }
              elseif ($age.Hours -gt 0) { "$($age.Hours)h atrás" }
              else { "$($age.Minutes)m atrás" }

    Write-Host "  [$i] $label  [$ageStr]$files" -ForegroundColor White
    $i++
}

Write-Host ""
$choice = Read-Host "Escolhe o número do backup a restaurar (ou ENTER para cancelar)"

if ([string]::IsNullOrWhiteSpace($choice)) {
    Write-Host "  Operação cancelada." -ForegroundColor DarkGray
    exit 0
}

$idx = [int]$choice - 1
if ($idx -lt 0 -or $idx -ge $backups.Count) {
    Write-Error "Número inválido: $choice"
    exit 1
}

$selected = $backups[$idx]
Write-Host ""
Write-Host "  Backup selecionado: $($selected.Name)" -ForegroundColor Yellow

# Confirmar
$confirm = Read-Host "Tens a certeza? O estado atual será substituído. (S/N)"
if ($confirm -notmatch "^[Ss]$") {
    Write-Host "  Operação cancelada." -ForegroundColor DarkGray
    exit 0
}

# Fazer backup automático do estado atual antes de restaurar
Write-Host ""
Write-Host ">>> A fazer backup de segurança do estado atual..." -ForegroundColor Yellow
& (Join-Path $PSScriptRoot "backup.ps1")

# Restaurar ficheiros
Write-Host ""
Write-Host ">>> A restaurar a partir de $($selected.Name)..." -ForegroundColor Yellow

$restored = 0
$files = @("settings.json", "favorites.json", "history.json")

foreach ($f in $files) {
    $src = Join-Path $selected.FullName $f
    if (Test-Path $src) {
        Copy-Item $src $repoRoot -Force
        Write-Host "  [OK]  $f restaurado" -ForegroundColor Green
        $restored++
    } else {
        Write-Host "  [--]  $f não existe neste backup" -ForegroundColor DarkGray
    }
}

# Restaurar binário release (opcional)
$srcExe = Join-Path $selected.FullName "sqlite-workbench-rs.exe"
if (Test-Path $srcExe) {
    $destExe = Join-Path $repoRoot "target\release\sqlite-workbench-rs.exe"
    $destDir = Split-Path $destExe
    if (-not (Test-Path $destDir)) { New-Item -ItemType Directory -Path $destDir -Force | Out-Null }
    Copy-Item $srcExe $destExe -Force
    Write-Host "  [OK]  Binário release restaurado" -ForegroundColor Green
    $restored++
}

Write-Host ""
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host "  RESTORE CONCLUÍDO — $restored ficheiro(s) restaurado(s)" -ForegroundColor Green
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host ""
