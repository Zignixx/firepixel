# Ende-zu-Ende-Test der MCP-Bruecke: unter der GUI-Sperre ausfuehren:
#   re\scripts\fwsteuerung\gui-sitzung.ps1 -Skript firepixel\tools\golden\fp-mcp\bruecke_e2e.ps1
# Danach: python -I bruecke_auswerten.py %TEMP%\firepixel-bruecke-e2e\antwort.jsonl %TEMP%\firepixel-bruecke-e2e
$ErrorActionPreference = 'Continue'
$env:FP_NO_SHELL_OPEN = '1'
$e2e = Join-Path $env:TEMP 'firepixel-bruecke-e2e'; New-Item -ItemType Directory -Force $e2e | Out-Null; $hier = 'C:\Users\Administrator\Desktop\REA\firepixel\tools\golden\fp-mcp'
$bin = 'C:\Users\Administrator\Desktop\REA\firepixel\target-au\debug'
$py = 'C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe'
$tok = Join-Path $env:LOCALAPPDATA 'Firepixel\mcp-bridge.json'
Remove-Item $tok -ErrorAction SilentlyContinue
$env:FIREPIXEL_MCP = '1'
$p = Start-Process -FilePath "$bin\firepixel.exe" -PassThru
$env:FIREPIXEL_MCP = $null
for ($i = 0; $i -lt 60 -and -not (Test-Path $tok); $i++) { Start-Sleep -Milliseconds 500 }
"Token-Datei da: $(Test-Path $tok)"
Start-Sleep -Seconds 2
& $py -I "$hier\bruecke_anfragen.py" $e2e | & "$bin\firepixel-cli.exe" mcp --bridge > "$e2e\antwort.jsonl"
"Exit cli: $LASTEXITCODE"
Start-Sleep -Seconds 1
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Remove-Item $tok -ErrorAction SilentlyContinue


