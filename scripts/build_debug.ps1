# ============================================================
#  Build Debug — SQLite Workbench RS
#  Gera o binário de desenvolvimento sem optimizações de release.
# ============================================================

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $repoRoot

$exe = Join-Path $repoRoot "target\debug\sqlite-workbench-rs.exe"

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  SQLite Workbench RS — DEBUG BUILD" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Verificar que cargo está disponível
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Cargo não encontrado. Instala o Rust em https://rustup.rs"
    exit 1
}

Write-Host ">>> Compilando em modo Debug..." -ForegroundColor Yellow
$sw = [System.Diagnostics.Stopwatch]::StartNew()

cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Error "Falha na compilação."
    exit 1
}

$sw.Stop()
$elapsed = $sw.Elapsed.ToString("mm\:ss")

Write-Host ""
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host "  BUILD CONCLUÍDO em $elapsed" -ForegroundColor Green
Write-Host "------------------------------------------------" -ForegroundColor Green

if (Test-Path $exe) {
    $size = (Get-Item $exe).Length / 1MB
    Write-Host "  Binário : $exe" -ForegroundColor Gray
    Write-Host "  Tamanho : $("{0:N1}" -f $size) MB" -ForegroundColor Gray
}

Write-Host ""
$ans = Read-Host "Executar a aplicação agora? (S/N)"
if ($ans -imatch "^s$") {
    Write-Host "A lançar..." -ForegroundColor Cyan
    Start-Process $exe -WorkingDirectory $repoRoot
}
