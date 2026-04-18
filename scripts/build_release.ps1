# ============================================================
#  Build Release — SQLite Workbench RS
#  Gera o binário optimizado com LTO, strip e codegen-units=1.
# ============================================================

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $repoRoot

$exe     = Join-Path $repoRoot "target\release\sqlite-workbench-rs.exe"
$distDir = Join-Path $repoRoot "dist"

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  SQLite Workbench RS — RELEASE BUILD" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Cargo não encontrado. Instala o Rust em https://rustup.rs"
    exit 1
}

# Configurar exclusão do Defender para a pasta target (evita locks durante link)
$targetRelease = Join-Path $repoRoot "target\release"
if (([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    try {
        Add-MpPreference -ExclusionPath $targetRelease -ErrorAction SilentlyContinue
        Write-Host ">>> Exclusão do Defender aplicada para o diretório target." -ForegroundColor DarkGray
    } catch { }
}

Write-Host ">>> Compilando em modo Release (LTO + strip)..." -ForegroundColor Yellow
$sw = [System.Diagnostics.Stopwatch]::StartNew()

cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Falha na compilação release."
    exit 1
}

$sw.Stop()
$elapsed = $sw.Elapsed.ToString("mm\:ss")

Write-Host ""
Write-Host "------------------------------------------------" -ForegroundColor Green
Write-Host "  RELEASE BUILD CONCLUÍDO em $elapsed" -ForegroundColor Green
Write-Host "------------------------------------------------" -ForegroundColor Green

if (Test-Path $exe) {
    $size = (Get-Item $exe).Length / 1MB
    Write-Host "  Binário : $exe" -ForegroundColor Gray
    Write-Host "  Tamanho : $("{0:N1}" -f $size) MB" -ForegroundColor Gray
}

# Copiar binário para /dist com timestamp
Write-Host ""
Write-Host ">>> A copiar para dist/..." -ForegroundColor Yellow
if (-not (Test-Path $distDir)) { New-Item -ItemType Directory -Path $distDir -Force | Out-Null }

$ts = Get-Date -Format "yyyyMMdd-HHmmss"
$destExe = Join-Path $distDir "sqlite-workbench-rs-$ts.exe"
Copy-Item $exe $destExe
Write-Host "  Copiado : $destExe" -ForegroundColor Gray

# Manter apenas os últimos 5 binários em dist/
$old = Get-ChildItem $distDir -Filter "sqlite-workbench-rs-*.exe" |
       Sort-Object LastWriteTime -Descending |
       Select-Object -Skip 5
if ($old) {
    $old | Remove-Item -Force
    Write-Host "  Limpos  : $($old.Count) binário(s) antigo(s)" -ForegroundColor DarkGray
}

Write-Host ""
$ans = Read-Host "Executar a aplicação agora? (S/N)"
if ($ans -imatch "^s$") {
    Write-Host "A lançar..." -ForegroundColor Cyan
    Start-Process $exe -WorkingDirectory $repoRoot
}
