# End-to-end test of the input injection (ui_input/ui_find) - run under the GUI lock:
#   re\scripts\fwsteuerung\gui-sitzung.ps1 -Skript firepixel\tools\golden\fp-mcp\bridge_input.ps1
# Result: %TEMP%\firepixel-bridge-input\result.json and images *.png
$ErrorActionPreference = 'Continue'
$env:FP_NO_SHELL_OPEN = '1'
$out = Join-Path $env:TEMP 'firepixel-bridge-input'; New-Item -ItemType Directory -Force $out | Out-Null
$here = 'C:\Users\Administrator\Desktop\REA\firepixel\tools\golden\fp-mcp'
$bin = 'C:\Users\Administrator\Desktop\REA\firepixel\target-au\debug'
$py = 'C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe'
$tok = Join-Path $env:LOCALAPPDATA 'Firepixel\mcp-bridge.json'
Remove-Item $tok -ErrorAction SilentlyContinue
$p = Start-Process -FilePath "$bin\firepixel.exe" -ArgumentList '--mcp' -PassThru
for ($i = 0; $i -lt 60 -and -not (Test-Path $tok); $i++) { Start-Sleep -Milliseconds 500 }
"Token file present: $(Test-Path $tok)"
Start-Sleep -Seconds 2
& $py -I -X utf8 "$here\bridge_input.py" $out *> "$out\log.txt"
"Exit python: $LASTEXITCODE"
Get-Content "$out\log.txt" -Encoding UTF8
Start-Sleep -Seconds 1
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Remove-Item $tok -ErrorAction SilentlyContinue
