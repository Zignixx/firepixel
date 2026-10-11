# Ende-zu-Ende-Test der Eingabe-Einspeisung (ui_input/ui_find) – unter der GUI-Sperre ausfuehren:
#   re\scripts\fwsteuerung\gui-sitzung.ps1 -Skript firepixel\tools\golden\fp-mcp\bruecke_eingabe.ps1
# Ergebnis: %TEMP%\firepixel-bruecke-eingabe\ergebnis.json und Bilder *.png
$ErrorActionPreference = 'Continue'
$env:FP_NO_SHELL_OPEN = '1'
$out = Join-Path $env:TEMP 'firepixel-bruecke-eingabe'; New-Item -ItemType Directory -Force $out | Out-Null
$hier = 'C:\Users\Administrator\Desktop\REA\firepixel\tools\golden\fp-mcp'
$bin = 'C:\Users\Administrator\Desktop\REA\firepixel\target-au\debug'
$py = 'C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe'
$tok = Join-Path $env:LOCALAPPDATA 'Firepixel\mcp-bridge.json'
Remove-Item $tok -ErrorAction SilentlyContinue
$p = Start-Process -FilePath "$bin\firepixel.exe" -ArgumentList '--mcp' -PassThru
for ($i = 0; $i -lt 60 -and -not (Test-Path $tok); $i++) { Start-Sleep -Milliseconds 500 }
"Token-Datei da: $(Test-Path $tok)"
Start-Sleep -Seconds 2
& $py -I -X utf8 "$hier\bruecke_eingabe.py" $out *> "$out\protokoll.txt"
"Exit python: $LASTEXITCODE"
Get-Content "$out\protokoll.txt" -Encoding UTF8
Start-Sleep -Seconds 1
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Remove-Item $tok -ErrorAction SilentlyContinue
