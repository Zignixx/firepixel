# End-to-end test of the MCP bridge: run under the GUI lock:
#   re\scripts\fwsteuerung\gui-sitzung.ps1 -Skript firepixel\tools\golden\fp-mcp\bridge_e2e.ps1
# Then: python -I bridge_evaluate.py %TEMP%\firepixel-bridge-e2e\response.jsonl %TEMP%\firepixel-bridge-e2e
$ErrorActionPreference = 'Continue'
$env:FP_NO_SHELL_OPEN = '1'
$e2e = Join-Path $env:TEMP 'firepixel-bridge-e2e'; New-Item -ItemType Directory -Force $e2e | Out-Null; $here = 'C:\Users\Administrator\Desktop\REA\firepixel\tools\golden\fp-mcp'
$bin = 'C:\Users\Administrator\Desktop\REA\firepixel\target-au\debug'
$py = 'C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe'
$tok = Join-Path $env:LOCALAPPDATA 'Firepixel\mcp-bridge.json'
Remove-Item $tok -ErrorAction SilentlyContinue
$env:FIREPIXEL_MCP = '1'
$p = Start-Process -FilePath "$bin\firepixel.exe" -PassThru
$env:FIREPIXEL_MCP = $null
for ($i = 0; $i -lt 60 -and -not (Test-Path $tok); $i++) { Start-Sleep -Milliseconds 500 }
"Token file present: $(Test-Path $tok)"
Start-Sleep -Seconds 2
& $py -I "$here\bridge_requests.py" $e2e | & "$bin\firepixel-cli.exe" mcp --bridge > "$e2e\response.jsonl"
"Exit cli: $LASTEXITCODE"
Start-Sleep -Seconds 1
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Remove-Item $tok -ErrorAction SilentlyContinue


