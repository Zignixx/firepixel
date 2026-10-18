# Parity runner - offline self test of the desktop host's key mapping (R-16).
# Loads orig_host.ps1 up to its command loop (C# block, key tables, functions) and runs `key_echo`'s code path in
# mock mode: no keyboard hook, no SendInput, no target program - the inputs that would be sent are echoed back.
# Prints vk / scan / extended per key in the host's current keyboard layout; exit 1 on any error or mismatch.
#   powershell -NoProfile -ExecutionPolicy Bypass -File selftest_host.ps1 [-Keys escape,delete,...]
param([string[]]$Keys = @('escape', 'delete', 'right', 'home', 'a', 'z', 'period'))
$ErrorActionPreference = 'Stop'
$host_src = Get-Content -Raw -Encoding UTF8 (Join-Path $PSScriptRoot 'orig_host.ps1')
$cut = $host_src.IndexOf('$PIPE_IN = ')
if ($cut -lt 0) { throw 'orig_host.ps1: command loop ($PIPE_IN) not found' }
. ([scriptblock]::Create($host_src.Substring(0, $cut)))
# mocks: no target process - the host's own keyboard layout, no window
function MainWin { [IntPtr]::Zero }
function TargetHkl { [PR.N]::GetKeyboardLayout(0) }
# R-16 regression: a caller's local $vk (same variable as the script's $VK table for PowerShell) must not reach VkOf
function ShadowProbe { $vk = [uint16]0; $vk = VkOf 'escape'; $vk = VkOf 'delete'; [int]$vk }
$fail = 0
try { if ((ShadowProbe) -ne 0x2E) { Write-Output 'FAIL: VkOf with a shadowing caller'; $fail++ } else { Write-Output 'ok: VkOf with a shadowing caller' } }
catch { Write-Output "FAIL: VkOf with a shadowing caller: $($_.Exception.Message)"; $fail++ }
# R-17: the focus / layout probe and the layout read-back run offline without a request (no `want`: nothing is posted)
try { $p = KeyProbe ''; $l = Layout ''; Write-Output ("ok: key_probe fg {0} focus {1} layout {2}; layout before {3} after {4} focus {5}" -f $p.fg_class, $p.focus_class, $p.layout, $l.before, $l.after, $l.focus_after)
      if ($p.requested -or $l.requested) { Write-Output 'FAIL: a layout request without want'; $fail++ } }
catch { Write-Output "FAIL: key_probe / layout: $($_.Exception.Message) [line $($_.InvocationInfo.ScriptLineNumber)]"; $fail++ }
$r = $null
try { $r = KeyEcho $Keys -mock } catch { Write-Output "FAIL: key_echo: $($_.Exception.Message) [line $($_.InvocationInfo.ScriptLineNumber)]"; exit 1 }
# expected: Escape always scan 0x01 without the extended flag; navigation/editing keys extended; letters/punctuation not
$extKeys = @('delete', 'del', 'insert', 'right', 'left', 'up', 'down', 'home', 'end', 'pageup', 'pagedown', 'arrowleft', 'arrowright', 'arrowup', 'arrowdown')
Write-Output ("layout {0} (host), events {1}" -f $r.layout, $r.events)
Write-Output ('{0,-8} {1,-6} {2,-6} {3,-4} {4,-12} {5,-5} {6}' -f 'key', 'vk', 'scan', 'ext', 'name', 'char', 'check')
foreach ($row in $r.keys) {
  $s = $row.sent; $why = @()
  if (-not $row.ok) { $why += 'echo mismatch' }
  if ($s.scan -eq 0) { $why += 'scan 0' }
  if (($s.ext -eq 1) -ne ($extKeys -contains ([string]$row.key).ToLower())) { $why += 'extended flag' }
  if (([string]$row.key).ToLower() -in @('escape', 'esc') -and $s.scan -ne 1) { $why += 'escape scan' }
  if ($why) { $fail++ }
  Write-Output ('{0,-8} 0x{1:X2}   0x{2:X2}   {3,-4} {4,-12} {5,-5} {6}' -f $row.key, $s.vk, $s.scan, $s.ext, $row.name, ($row.char -replace '[\x00-\x1f]', '?'), $(if ($why) { 'FAIL: ' + ($why -join ', ') } else { 'ok' }))
}
if ($fail) { Write-Output "$fail failure(s)"; exit 1 }
Write-Output 'all ok'
exit 0
