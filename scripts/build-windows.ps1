# Compila a Rádio Escolar no Windows (requer Rust, Node 20+ e VS Build Tools C++).
# Uso: abra o PowerShell na pasta do projeto e rode:  .\scripts\build-windows.ps1
$ErrorActionPreference = "Stop"
Write-Host "1/4 Instalando dependências da interface..." -ForegroundColor Cyan
npm install
Write-Host "2/4 Testando o agendador..." -ForegroundColor Cyan
cargo test -p radio-core
Write-Host "3/4 Compilando e gerando instaladores (pode demorar na primeira vez)..." -ForegroundColor Cyan
npx tauri build
Write-Host "4/4 Montando versão portátil..." -ForegroundColor Cyan
$exe = Get-ChildItem "target/release" -Filter *.exe | Where-Object { $_.Name -notlike "*build*" } | Select-Object -First 1
New-Item -ItemType Directory -Force -Path "portatil" | Out-Null
Copy-Item $exe.FullName "portatil/Radio Escolar MJS.exe" -Force
"Versão portátil: os dados ficam na pasta 'dados' ao lado deste arquivo." | Out-File -Encoding utf8 "portatil/portatil.txt"
Compress-Archive -Path "portatil/*" -DestinationPath "Radio-Escolar-MJS-portatil.zip" -Force
Write-Host ""
Write-Host "Pronto!" -ForegroundColor Green
Write-Host "Instalador: target\release\bundle\nsis\"
Write-Host "MSI:        target\release\bundle\msi\"
Write-Host "Portátil:   Radio-Escolar-MJS-portatil.zip"
