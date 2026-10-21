# Parity runner - offline self test of the desktop host's key mapping (R-16).
# Loads orig_host.ps1 up to its command loop (C# block, key tables, functions) and runs `key_echo`'s code path in
# mock mode: no keyboard hook, no SendInput, no target program - the inputs that would be sent are echoed back.
# Prints vk / scan / extended per key in the host's current keyboard layout for both key paths (R-27: scan code only
# for unmodified keys, vk + scan code for modified keys); exit 1 on any error or mismatch.
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
# R-27 (TW-179) regression: a single named key must stay the whole name (it was its last letter: escape -> E)
try { $bad = @()
  foreach ($t in @(@('escape', '', 'escape'), @('Enter', '', 'Enter'), @('t', '', 't'), @('ctrl+shift+z', 'ctrl,shift', 'z'), @('shift+right', 'shift', 'right'))) {
    $cp = ComboParts $t[0]; $m = (@($cp.mods) -join ','); if ($cp.main -ne $t[2] -or $m -ne $t[1]) { $bad += "$($t[0]) -> mods '$m' main '$($cp.main)'" } }
  if ($bad) { Write-Output "FAIL: combo parts: $($bad -join '; ')"; $fail++ } else { Write-Output 'ok: combo parts (single named keys stay whole)' } }
catch { Write-Output "FAIL: combo parts: $($_.Exception.Message) [line $($_.InvocationInfo.ScriptLineNumber)]"; $fail++ }
# R-17: the focus / layout probe and the layout read-back run offline without a request (no `want`: nothing is posted)
try { $p = KeyProbe ''; $l = Layout ''; Write-Output ("ok: key_probe fg {0} focus {1} layout {2}; layout before {3} after {4} focus {5}" -f $p.fg_class, $p.focus_class, $p.layout, $l.before, $l.after, $l.focus_after)
      if ($p.requested -or $l.requested) { Write-Output 'FAIL: a layout request without want'; $fail++ } }
catch { Write-Output "FAIL: key_probe / layout: $($_.Exception.Message) [line $($_.InvocationInfo.ScriptLineNumber)]"; $fail++ }
# R-20 (TW-146): typed characters as real keys (`type` with mode keys): vk, scan code and shift state per character
try { $bad = @(); $row = @()
  foreach ($ch in 'Ab 1'.ToCharArray()) { $a = [PR.N]::CharKeys($ch); if ($a -eq $null) { $bad += "'$ch' no key"; continue }
    $main = @($a | ? { $_.u.ki.wVk -ne 0x10 -and $_.u.ki.wVk -ne 0x11 -and $_.u.ki.wVk -ne 0x12 })[0]; $sh = @($a | ? { $_.u.ki.wVk -eq 0x10 }).Count -gt 0
    if ($main.u.ki.wScan -eq 0) { $bad += "'$ch' scan 0" }; if ([char]::IsUpper($ch) -ne $sh) { $bad += "'$ch' shift $sh" }
    $row += ('{0}=0x{1:X2}/0x{2:X2}{3}' -f $ch, $main.u.ki.wVk, $main.u.ki.wScan, $(if ($sh) { '+shift' } else { '' })) }
  if ($bad) { Write-Output "FAIL: typed keys: $($bad -join ', ')"; $fail++ } else { Write-Output "ok: typed keys $($row -join ' ')" } }
catch { Write-Output "FAIL: typed keys: $($_.Exception.Message) [line $($_.InvocationInfo.ScriptLineNumber)]"; $fail++ }
# R-27 (TW-179): typed characters on the scan-code path - every input scan code only (wVk 0, KEYEVENTF_SCANCODE)
try { $bad = @(); $row = @()
  foreach ($ch in 'Ab 1'.ToCharArray()) { $a = [PR.N]::CharKeys($ch, $true); if ($a -eq $null) { $bad += "'$ch' no key"; continue }
    foreach ($in in $a) { if ($in.u.ki.wVk -ne 0 -or ($in.u.ki.dwFlags -band 8) -eq 0 -or $in.u.ki.wScan -eq 0) { $bad += "'$ch' input vk $($in.u.ki.wVk) scan $($in.u.ki.wScan) flags $($in.u.ki.dwFlags)" } }
    $sh = @($a | ? { $_.u.ki.wScan -eq 0x2A }).Count -gt 0; if ([char]::IsUpper($ch) -ne $sh) { $bad += "'$ch' shift $sh" }
    $row += ('{0}={1}' -f $ch, (@($a | % { '0x{0:X2}{1}' -f $_.u.ki.wScan, $(if ($_.u.ki.dwFlags -band 2) { 'u' } else { 'd' }) }) -join '/')) }
  if ($bad) { Write-Output "FAIL: typed scan-code keys: $($bad -join ', ')"; $fail++ } else { Write-Output "ok: typed scan-code keys $($row -join ' ')" } }
catch { Write-Output "FAIL: typed scan-code keys: $($_.Exception.Message) [line $($_.InvocationInfo.ScriptLineNumber)]"; $fail++ }
# expected: Escape always scan 0x01 without the extended flag; navigation/editing keys extended; letters/punctuation not.
# R-27: both paths - 'scancode' (unmodified keys: wVk 0 sent, the vk comes from the system) and 'vk' (modified keys)
$extKeys = @('delete', 'del', 'insert', 'right', 'left', 'up', 'down', 'home', 'end', 'pageup', 'pagedown', 'arrowleft', 'arrowright', 'arrowup', 'arrowdown')
foreach ($path in 'scancode', 'vk') {
  $r = $null
  try { $r = KeyEcho $Keys -mock -path $path } catch { Write-Output "FAIL: key_echo ($path): $($_.Exception.Message) [line $($_.InvocationInfo.ScriptLineNumber)]"; exit 1 }
  Write-Output ("path {0}: layout {1} (host), events {2}" -f $r.path, $r.layout, $r.events)
  Write-Output ('{0,-8} {1,-6} {2,-6} {3,-4} {4,-5} {5,-12} {6,-5} {7}' -f 'key', 'vk', 'scan', 'ext', 'wVk', 'name', 'char', 'check')
  foreach ($row in $r.keys) {
    $s = $row.sent; $why = @()
    if (-not $row.ok) { $why += 'echo mismatch' }
    if ($s.scan -eq 0) { $why += 'scan 0' }
    if ($row.path -ne $path) { $why += "path $($row.path)" }
    if ($path -eq 'scancode' -and $s.wvk -ne 0) { $why += 'wVk not 0' }
    if ($path -eq 'vk' -and $s.wvk -ne $s.vk) { $why += 'wVk not the vk' }
    if (($s.ext -eq 1) -ne ($extKeys -contains ([string]$row.key).ToLower())) { $why += 'extended flag' }
    if (([string]$row.key).ToLower() -in @('escape', 'esc') -and $s.scan -ne 1) { $why += 'escape scan' }
    if ($why) { $fail++ }
    Write-Output ('{0,-8} 0x{1:X2}   0x{2:X2}   {3,-4} 0x{4:X2}  {5,-12} {6,-5} {7}' -f $row.key, $s.vk, $s.scan, $s.ext, $s.wvk, $row.name, ($row.char -replace '[\x00-\x1f]', '?'), $(if ($why) { 'FAIL: ' + ($why -join ', ') } else { 'ok' }))
  }
}
if ($fail) { Write-Output "$fail failure(s)"; exit 1 }
Write-Output 'all ok'
exit 0
