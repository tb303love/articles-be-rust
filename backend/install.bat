@echo off
:: 1. Pokrećemo tvoju kompletnu PowerShell skriptu da pripremi Angular i Rust u Release modu
echo [1/3] Pokrecem kompletan Angular i Rust produkcioni build...
:: Pozivamo PowerShell i nalažemo mu da izvrši tvoju build skriptu
powershell -NoProfile -ExecutionPolicy Bypass -File .\build_all.ps1

:: Ako je build_all.ps1 uspešno završio, čistimo i pripremamo WiX strukturu
echo Ciscenje WiX keša...
cargo wix clean

:: 2. Kompajliranje novog MSI-ja (Sve varijable vuče automatski iz Cargo.toml i main.wxs)
echo [2/3] Kreiranje MSI instalera preko WiX Toolset-a...
cargo wix --nocapture

echo [3/3] Gotovo! Instaler se nalazi u: target\wix\
pause
