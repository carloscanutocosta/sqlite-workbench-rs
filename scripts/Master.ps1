# ============================================================
#  Master Script — SQLite Workbench RS
#  Executa a compilação de release e o backup sequencialmente.
# ============================================================

param (
    [string]$Path = "D:\Backup\sqlite_workbench"
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  SQLite Workbench RS — MASTER WORKFLOW" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# 1. Executar o Build de Release
Write-Host ">>> ETAPA 1: Compilação de Release" -ForegroundColor Yellow
& (Join-Path $PSScriptRoot "build_release.ps1")

# Atualizar a data do último build no README.md
$readmeFile = Join-Path $PSScriptRoot "README.md"
if (Test-Path $readmeFile) {
    $now = Get-Date -Format "yyyy-MM-dd HH:mm"
    (Get-Content $readmeFile) -replace "^\*Último build realizado em:\* .*$", "*Último build realizado em:* $now" | 
        Set-Content $readmeFile -Encoding UTF8
    Write-Host ">>> README.md atualizado com a data: $now" -ForegroundColor Gray
}

# 2. Executar o Backup
# Nota: O backup.ps1 irá incluir o novo binário gerado na etapa anterior.
Write-Host "`n>>> ETAPA 2: Backup automático" -ForegroundColor Yellow
& (Join-Path $PSScriptRoot "backup.ps1") -Path $Path

Write-Host ""
Write-Host "[CONCLUÍDO] Build e Backup terminados com sucesso." -ForegroundColor Green