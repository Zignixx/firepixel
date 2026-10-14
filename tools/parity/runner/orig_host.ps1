# Parity runner - desktop host for the reference program.
# Reads one JSON command per line on stdin, answers one JSON line on stdout:
#   {"id":1,"cmd":"windows"}  ->  {"id":1,"ok":true,"result":...}
# Target process, main/view window classes come from the runner (runner.config.json), never from here.
# Real input (SendInput) is only sent when the foreground window belongs to the target process
# (and, if configured, has the main window class); otherwise the command fails.
$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = New-Object Text.UTF8Encoding $false
[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text; using System.Drawing;
namespace PR {
public static class N {
  [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Explicit)] public struct U { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
  [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public U u; }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public delegate bool CB(IntPtr h, IntPtr l);
  [DllImport("user32.dll", SetLastError=true)] public static extern uint SendInput(uint n, INPUT[] i, int cb);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hh, bool r);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(CB cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, CB cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint c);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetMenuItemCount(IntPtr m);
  [DllImport("user32.dll")] public static extern IntPtr GetSubMenu(IntPtr m, int i);
  [DllImport("user32.dll")] public static extern uint GetMenuState(IntPtr m, uint i, uint f);
  [DllImport("user32.dll")] public static extern bool GetMenuItemRect(IntPtr h, IntPtr m, uint i, out RECT r);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetMenuString(IntPtr m, uint i, StringBuilder s, int n, uint f);
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="SendMessageW")] public static extern IntPtr SendMessageS(IntPtr h, uint m, IntPtr w, string l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="SendMessageW")] public static extern IntPtr SendMessageSB(IntPtr h, uint m, IntPtr w, StringBuilder l);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern IntPtr ChildWindowFromPointEx(IntPtr h, POINT p, uint f);
  public static string MenuText(IntPtr m, int i) { var s = new StringBuilder(256); GetMenuString(m, (uint)i, s, 256, 0x400); return s.ToString(); }
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern void keybd_event(byte k, byte s, uint f, IntPtr e);
  public static string Cls(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
  public static string Txt(IntPtr h) { var s = new StringBuilder(512); GetWindowText(h, s, 512); return s.ToString(); }
  public static uint Pid(IntPtr h) { uint p; GetWindowThreadProcessId(h, out p); return p; }
  public static List<IntPtr> Tops(uint pid) { var l = new List<IntPtr>(); EnumWindows((h, x) => { if (IsWindowVisible(h) && Pid(h) == pid) l.Add(h); return true; }, IntPtr.Zero); return l; }
  public static List<IntPtr> Kids(IntPtr p) { var l = new List<IntPtr>(); EnumChildWindows(p, (h, x) => { if (IsWindowVisible(h)) l.Add(h); return true; }, IntPtr.Zero); return l; }
  public static uint TargetPid = 0; public static string TargetClass = ""; public static IntPtr MainHwnd = IntPtr.Zero;
  public static string Guard() { if (TargetPid == 0) return "no target"; IntPtr f = GetForegroundWindow(); if (f == IntPtr.Zero) return "no foreground window";
    if (Pid(f) != TargetPid) return "foreground belongs to process " + Pid(f) + " (" + Cls(f) + ")";
    // own windows of the target process only: main window, dialogs, popup menus, MFC tool windows, panels
    // (OWL.*) and the Flash based panels (SWFPLAYER*)
    string c = Cls(f); if (TargetClass != "" && c != TargetClass && c != "#32770" && c != "#32768" && !c.StartsWith("Afx:") && !c.StartsWith("OWL.") && !c.StartsWith("SWFPLAYER")) return "foreground class " + c; return ""; }
  // The foreground is briefly empty while a dialog/menu closes or a window is activated: wait for it
  // (up to ~2 s); when it stays empty, give it back to the target's main window. A foreground of another
  // process is never overridden here.
  public static void Send(INPUT[] a) { string g = Guard();
    for (int k = 0; k < 20 && g != ""; k++) {
      if (g == "no foreground window" && MainHwnd != IntPtr.Zero && k == 6) { keybd_event(0x12, 0, 0, IntPtr.Zero); keybd_event(0x12, 0, 2, IntPtr.Zero); SetForegroundWindow(MainHwnd); }
      if (g != "no foreground window" && !g.StartsWith("foreground class")) break;
      System.Threading.Thread.Sleep(100); g = Guard(); }
    if (g != "") throw new Exception("input refused: " + g);
    uint r = SendInput((uint)a.Length, a, Marshal.SizeOf(typeof(INPUT))); if (r != a.Length) throw new Exception("SendInput " + r + "/" + a.Length); }
  [StructLayout(LayoutKind.Sequential)] public struct GUITHREADINFO { public int cbSize, flags; public IntPtr hwndActive, hwndFocus, hwndCapture, hwndMenuOwner, hwndMoveSize, hwndCaret; public RECT rcCaret; }
  [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint tid, ref GUITHREADINFO g);
  public static IntPtr FocusOf(IntPtr top) { uint p; uint tid = GetWindowThreadProcessId(top, out p); var g = new GUITHREADINFO(); g.cbSize = Marshal.SizeOf(typeof(GUITHREADINFO)); if (!GetGUIThreadInfo(tid, ref g)) return IntPtr.Zero; return g.hwndFocus; }
  public static INPUT Key(ushort vk, bool up) { var i = new INPUT(); i.type = 1; i.u.ki.wVk = vk; i.u.ki.dwFlags = up ? 2u : 0u;
    if ((vk >= 0x21 && vk <= 0x28) || vk == 0x2D || vk == 0x2E) i.u.ki.dwFlags |= 1; return i; }
  public static INPUT Uni(char c, bool up) { var i = new INPUT(); i.type = 1; i.u.ki.wScan = c; i.u.ki.dwFlags = 4u | (up ? 2u : 0u); return i; }
  public static INPUT Mouse(uint flags, uint data) { var i = new INPUT(); i.type = 0; i.u.mi.dwFlags = flags; i.u.mi.mouseData = data; return i; }
  public static Bitmap Screen(int l, int t, int r, int b) { var bmp = new Bitmap(Math.Max(1, r - l), Math.Max(1, b - t)); using (var g = Graphics.FromImage(bmp)) g.CopyFromScreen(l, t, 0, 0, bmp.Size); return bmp; }
}}
'@
[PR.N]::SetProcessDPIAware() | Out-Null
$UIA = [System.Windows.Automation.AutomationElement]
$TW = [System.Windows.Automation.TreeWalker]::ControlViewWalker
$S = @{ process = ''; main_class = ''; view_class = ''; panel_class = ''; pid = 0; forbidden = @() }

# LRESULT of a 32-bit target arrives zero-extended (-1 = 4294967295): back to a signed 32-bit value
function I32($v) { $x = [int64]$v.ToInt64(); if ($x -gt [int64][int]::MaxValue) { $x -= [int64]4294967296 }; [int]$x }
function R4($r) { @([int]$r.L, [int]$r.T, [int]$r.R, [int]$r.B) }
function WRect($h) { $r = New-Object PR.N+RECT; [PR.N]::GetWindowRect($h, [ref]$r) | Out-Null; R4 $r }
function CRect($h) { $r = New-Object PR.N+RECT; [PR.N]::GetClientRect($h, [ref]$r) | Out-Null; $p = New-Object PR.N+POINT; [PR.N]::ClientToScreen($h, [ref]$p) | Out-Null; @($p.X, $p.Y, ($p.X + $r.R), ($p.Y + $r.B)) }
# Several processes of the program may exist (a script launcher waiting for a busy instance, a hung
# restart): the target is the process owning the visible main window (by class, across processes);
# without one the process with any visible window, else the first one.
function MainOwner { $best = 0; $area = 0
  if (-not $S.main_class) { return 0 }
  foreach ($p in @(Get-Process -Name $S.process -ErrorAction SilentlyContinue)) {
    foreach ($h in [PR.N]::Tops([uint32]$p.Id)) { if ([PR.N]::Cls($h) -ne $S.main_class) { continue }
      $r = WRect $h; $a = ($r[2] - $r[0]) * ($r[3] - $r[1]); if ($a -gt $area) { $area = $a; $best = $p.Id } } }
  $best }
function Proc {
  $all = @(Get-Process -Name $S.process -ErrorAction SilentlyContinue)
  $p = $null; $mo = MainOwner
  if ($mo) { $p = $all | ? { $_.Id -eq $mo } | Select-Object -First 1 }
  if (-not $p) { $p = $all | ? { [PR.N]::Tops([uint32]$_.Id).Count -gt 0 } | Select-Object -First 1 }
  if (-not $p) { $p = $all | Select-Object -First 1 }
  if ($p) { $S.pid = $p.Id; [PR.N]::TargetPid = [uint32]$p.Id } else { $S.pid = 0; [PR.N]::TargetPid = 0 }; $p }
function ProcList { $mo = MainOwner; $cl = @{}
  try { foreach ($w in @(Get-CimInstance Win32_Process -Filter "Name='$($S.process).exe'" -ErrorAction Stop)) { $cl[[int]$w.ProcessId] = [string]$w.CommandLine } } catch { }
  @(Get-Process -Name $S.process -ErrorAction SilentlyContinue | % { $age = -1; try { $age = [int]((Get-Date) - $_.StartTime).TotalSeconds } catch { }
    $tops = @([PR.N]::Tops([uint32]$_.Id))
    @{ pid = $_.Id; has_main = ($_.Id -eq $mo); windows = $tops.Count; dialogs = @($tops | ? { [PR.N]::Cls($_) -eq '#32770' } | % { "$([PR.N]::Txt($_)) | $(DialogTexts $_)" }); age_s = $age; cmdline = $cl[[int]$_.Id] } }) }
function MainWin {
  if (-not (Proc)) { throw 'process not running' }
  $best = [IntPtr]::Zero; $area = 0
  foreach ($h in [PR.N]::Tops([uint32]$S.pid)) {
    $c = [PR.N]::Cls($h)
    if ($S.main_class -and $c -ne $S.main_class) { continue }
    if ($c -in '#32770', '#32768', 'tooltips_class32') { continue }
    $r = WRect $h; $a = ($r[2] - $r[0]) * ($r[3] - $r[1]); if ($a -gt $area) { $area = $a; $best = $h }
  }
  if ($best -eq [IntPtr]::Zero) { throw 'main window not found' }
  $best
}
function WinInfo($h) { @{ hwnd = [int64]$h; class = [PR.N]::Cls($h); title = [PR.N]::Txt($h); rect = (WRect $h) } }
function Union($rects) { if (-not $rects) { return $null }; $l = ($rects | % { $_[0] } | Measure -Min).Minimum; $t = ($rects | % { $_[1] } | Measure -Min).Minimum; $r = ($rects | % { $_[2] } | Measure -Max).Maximum; $b = ($rects | % { $_[3] } | Measure -Max).Maximum; @([int]$l, [int]$t, [int]$r, [int]$b) }
function Clean([string]$s) { ($s -replace '&', '' -replace "`t.*$", '' -replace '\.\.\.$|…$', '').Trim() }
# modal windows: standard dialogs and the program's Flash based dialogs (e.g. file info), which are top-level
# windows of the Flash player class without Win32 buttons (closed with WM_CLOSE = cancel)
$FLASHDLG = 'SWFPLAYER_SWFWINDOWCONTAINER'
function Dialogs { if (-not (Proc)) { throw 'process not running' }; [PR.N]::Tops([uint32]$S.pid) | ? { $k = [PR.N]::Cls($_); $k -eq '#32770' -or $k -eq $FLASHDLG } }
function CloseFlash { $n = 0; foreach ($h in @([PR.N]::Tops([uint32]$S.pid) | ? { [PR.N]::Cls($_) -eq $FLASHDLG })) { [PR.N]::PostMessage($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null; $n++ }
  if ($n) { Start-Sleep -Milliseconds 500 }; $n }
function DialogTexts($h) { (([PR.N]::Kids($h) | % { [PR.N]::Txt($_) } | ? { $_ }) -join ' | ') }

function Focus {
  $h = MainWin
  [PR.N]::TargetClass = $S.main_class; [PR.N]::MainHwnd = $h
  if ([PR.N]::IsIconic($h)) { [PR.N]::ShowWindow($h, 9) | Out-Null }
  # a dialog of the target may legitimately own the foreground
  $f = [PR.N]::GetForegroundWindow()
  if ($f -ne [IntPtr]::Zero -and [PR.N]::Pid($f) -eq [uint32]$S.pid) { return @{ fg = [int64]$f; class = [PR.N]::Cls($f) } }
  [PR.N]::keybd_event(0x12, 0, 0, [IntPtr]::Zero); [PR.N]::keybd_event(0x12, 0, 2, [IntPtr]::Zero)
  [PR.N]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 250
  $f = [PR.N]::GetForegroundWindow()
  @{ fg = [int64]$f; class = [PR.N]::Cls($f); ok = ([PR.N]::Pid($f) -eq [uint32]$S.pid) }
}

$VK = @{ shift = 0x10; ctrl = 0x11; alt = 0x12; space = 0x20; enter = 0x0D; return = 0x0D; esc = 0x1B; escape = 0x1B; tab = 0x09; backspace = 0x08; back = 0x08; delete = 0x2E; del = 0x2E; insert = 0x2D
  left = 0x25; up = 0x26; right = 0x27; down = 0x28; arrowleft = 0x25; arrowup = 0x26; arrowright = 0x27; arrowdown = 0x28; home = 0x24; end = 0x23; pageup = 0x21; pagedown = 0x22
  plus = 0xBB; minus = 0xBD; comma = 0xBC; period = 0xBE }
function VkOf([string]$k) { $k = $k.ToLower(); if ($VK.ContainsKey($k)) { return [uint16]$VK[$k] }; if ($k.Length -eq 1) { return [uint16][int][char]$k.ToUpper() }; if ($k -match '^f(\d+)$') { return [uint16](0x6F + [int]$matches[1]) }; if ($k -match '^0x') { return [uint16][Convert]::ToInt32($k, 16) }; throw "unknown key $k" }
function Combo([string]$c) {
  $parts = $c.Split('+') | ? { $_ }
  if (($parts | % { $_.ToLower() }) -contains 'ctrl' -and ($parts | % { $_.ToLower() }) -contains 'alt' -and ($parts | % { $_.ToLower() }) -contains 'space') { throw 'ctrl+alt+space is forbidden' }
  $mods = @(); if ($parts.Count -gt 1) { $mods = $parts[0..($parts.Count - 2)] }; $main = $parts[-1]
  $a = @(); foreach ($m in $mods) { $a += [PR.N]::Key((VkOf $m), $false) }
  $a += [PR.N]::Key((VkOf $main), $false); $a += [PR.N]::Key((VkOf $main), $true)
  for ($i = $mods.Count - 1; $i -ge 0; $i--) { $a += [PR.N]::Key((VkOf $mods[$i]), $true) }
  [PR.N]::Send($a)
}
function Mods($mods, [bool]$up) { $a = @(); foreach ($m in @($mods)) { if ($m) { $a += [PR.N]::Key((VkOf $m), $up) } }; if ($a.Count) { [PR.N]::Send($a) } }
function Click($x, $y, $button, $mods, $count) {
  [PR.N]::SetCursorPos([int]$x, [int]$y) | Out-Null; Start-Sleep -Milliseconds 60
  [PR.N]::Send(@([PR.N]::Mouse(0x1, 0)))
  Mods $mods $false
  $d, $u = 0x2, 0x4; if ($button -eq 'right') { $d, $u = 0x8, 0x10 } elseif ($button -eq 'middle') { $d, $u = 0x20, 0x40 }
  for ($i = 0; $i -lt [Math]::Max(1, [int]$count); $i++) { [PR.N]::Send(@([PR.N]::Mouse($d, 0))); Start-Sleep -Milliseconds 70; [PR.N]::Send(@([PR.N]::Mouse($u, 0))); Start-Sleep -Milliseconds 50 }
  Mods $mods $true; Start-Sleep -Milliseconds 120
}
function Drag($x1, $y1, $x2, $y2, $mods, $steps, $button) {
  if (-not $steps) { $steps = 20 }
  $d, $u = 0x2, 0x4; if ($button -eq 'right') { $d, $u = 0x8, 0x10 }
  [PR.N]::SetCursorPos([int]$x1, [int]$y1) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 80
  Mods $mods $false
  [PR.N]::Send(@([PR.N]::Mouse($d, 0))); Start-Sleep -Milliseconds 60
  for ($i = 1; $i -le $steps; $i++) {
    [PR.N]::SetCursorPos([int]($x1 + ($x2 - $x1) * $i / $steps), [int]($y1 + ($y2 - $y1) * $i / $steps)) | Out-Null
    [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 20
  }
  [PR.N]::Send(@([PR.N]::Mouse($u, 0))); Start-Sleep -Milliseconds 60
  Mods $mods $true; Start-Sleep -Milliseconds 150
}

# ------------------------------------------------------------------ UI Automation
function El([IntPtr]$h) { $UIA::FromHandle($h) }
function ElInfo($e) { $i = $e.Current; $b = $i.BoundingRectangle
  @{ name = $i.Name; type = $i.ControlType.ProgrammaticName -replace '^ControlType\.', ''; class = $i.ClassName; auto_id = $i.AutomationId; enabled = $i.IsEnabled
     rect = $(if ($b.IsEmpty) { $null } else { @([int]$b.Left, [int]$b.Top, [int]$b.Right, [int]$b.Bottom) }) } }
function Roots($scope) {
  $m = MainWin
  if ($scope -eq 'main') { return @(El $m) }
  # main window first, then other top-level windows of the process (dialogs, floating panels, menus)
  $l = @(El $m); foreach ($h in [PR.N]::Tops([uint32]$S.pid)) { if ($h -ne $m) { $l += El $h } }; $l
}
function Match($e, $name, $type, $exact) {
  $i = $e.Current
  if ($type -and ($i.ControlType.ProgrammaticName -replace '^ControlType\.', '') -ne $type) { return $false }
  if (-not $name) { return $true }
  $n = Clean $i.Name; $q = Clean $name
  if ($exact) { return $n -eq $q }
  return ($n -eq $q) -or ($n.StartsWith($q, [StringComparison]::OrdinalIgnoreCase))
}
function FindEl($name, $type, $scope, $exact, $depth) {
  if (-not $depth) { $depth = 12 }
  foreach ($r in (Roots $scope)) {
    $cond = [System.Windows.Automation.Condition]::TrueCondition
    $all = $r.FindAll([System.Windows.Automation.TreeScope]::Subtree, $cond)
    $hit = $null
    foreach ($e in $all) { if (Match $e $name $type $true) { if (-not $e.Current.IsOffscreen) { return $e }; if (-not $hit) { $hit = $e } } }
    if ($hit) { return $hit }
    if (-not $exact) { foreach ($e in $all) { if (Match $e $name $type $false) { return $e } } }
  }
  $null
}
function Dump($e, $d, $max, $out) { if ($d -gt $max -or $out.Count -gt 1500) { return }; $c = $TW.GetFirstChild($e)
  while ($c) { $x = ElInfo $c; $x.depth = $d; [void]$out.Add($x); Dump $c ($d + 1) $max $out; $c = $TW.GetNextSibling($c) } }
function Pattern($e, $p) { $o = $null; if ($e.TryGetCurrentPattern($p, [ref]$o)) { return $o }; $null }
function DoAct($e, $act, $value) {
  switch ($act) {
    'invoke' { $p = Pattern $e ([System.Windows.Automation.InvokePattern]::Pattern); if ($p) { $p.Invoke(); return 'invoked' }
               $p = Pattern $e ([System.Windows.Automation.ExpandCollapsePattern]::Pattern); if ($p) { $p.Expand(); return 'expanded' }
               $p = Pattern $e ([System.Windows.Automation.SelectionItemPattern]::Pattern); if ($p) { $p.Select(); return 'selected' }
               $p = Pattern $e ([System.Windows.Automation.TogglePattern]::Pattern); if ($p) { $p.Toggle(); return 'toggled' }
               throw 'no invoke pattern' }
    'expand' { $p = Pattern $e ([System.Windows.Automation.ExpandCollapsePattern]::Pattern); if (-not $p) { throw 'no expand pattern' }; $p.Expand(); return 'expanded' }
    'collapse' { $p = Pattern $e ([System.Windows.Automation.ExpandCollapsePattern]::Pattern); if (-not $p) { throw 'no collapse pattern' }; $p.Collapse(); return 'collapsed' }
    'select' { $p = Pattern $e ([System.Windows.Automation.SelectionItemPattern]::Pattern); if (-not $p) { throw 'no select pattern' }; $p.Select(); return 'selected' }
    'toggle' { $p = Pattern $e ([System.Windows.Automation.TogglePattern]::Pattern); if (-not $p) { throw 'no toggle pattern' }; $p.Toggle(); return 'toggled' }
    'focus' { $e.SetFocus(); return 'focused' }
    'set' { $p = Pattern $e ([System.Windows.Automation.ValuePattern]::Pattern); if (-not $p) { throw 'no value pattern' }; $p.SetValue([string]$value); return 'set' }
    default { throw "unknown uia action $act" }
  }
}

# menus: the menu bar is custom drawn; it is opened with the keyboard (Alt, Right x n, Down).
# Open popup menus are standard #32768 windows: MN_GETHMENU gives their items, texts, states and rects.
function MenuWins { @([PR.N]::Tops([uint32]$S.pid) | ? { [PR.N]::Cls($_) -eq '#32768' }) }
function PopupItems($w) {
  $m = [PR.N]::SendMessage($w, 0x01E1, [IntPtr]::Zero, [IntPtr]::Zero)
  $items = @()
  if ($m -eq [IntPtr]::Zero) { return }
  $n = [PR.N]::GetMenuItemCount($m)
  for ($i = 0; $i -lt $n; $i++) {
    $st = [PR.N]::GetMenuState($m, [uint32]$i, 0x400); $r = New-Object PR.N+RECT; $sub = ([PR.N]::GetSubMenu($m, $i) -ne [IntPtr]::Zero); if ($sub) { $st = $st -band 0xFF }
    [PR.N]::GetMenuItemRect([IntPtr]::Zero, $m, [uint32]$i, [ref]$r) | Out-Null
    $raw = [PR.N]::MenuText($m, $i)
    @{ index = $i; text = (Clean $raw); raw = $raw; separator = ((-not $sub) -and [bool]($st -band 0x800)); grayed = [bool]($st -band 0x3); checked = [bool]($st -band 0x8); submenu = $sub; rect = (R4 $r) }
  }
}
function TreeOf($m, $pre, $out) { $n = [PR.N]::GetMenuItemCount($m); for ($i = 0; $i -lt $n; $i++) { $t = Clean ([PR.N]::MenuText($m, $i)); if (-not $t) { continue }
  $p = if ($pre) { "$pre > $t" } else { $t }; [void]$out.Add($p); $sm = [PR.N]::GetSubMenu($m, $i); if ($sm -ne [IntPtr]::Zero -and $out.Count -lt 3000) { TreeOf $sm $p $out } } }
function OpenMenus { $l = New-Object Collections.ArrayList; foreach ($w in (MenuWins)) { [void]$l.Add(@{ hwnd = [int64]$w; rect = (WRect $w); items = @(PopupItems $w) }) }; @{ menus = $l.ToArray(); count = $l.Count } }
function CloseMenus {
  for ($k = 0; $k -lt 6 -and (MenuWins).Count; $k++) {
    foreach ($w in (MenuWins)) { [PR.N]::PostMessage($w, 0x100, [IntPtr]0x1B, [IntPtr]0x00010001) | Out-Null; [PR.N]::PostMessage($w, 0x101, [IntPtr]0x1B, [IntPtr]([int64]0xC0010001)) | Out-Null }
    Start-Sleep -Milliseconds 200
  }
  (MenuWins).Count }
function MenuPath($path, $final, $bar) {
  # a modal window left open (Flash dialog, message box) blocks the menu bar: close it first
  CloseFlash | Out-Null
  if ((Focus).ok -eq $false) { throw 'cannot focus target' }
  if ((CloseMenus) -gt 0) { throw 'open menu could not be closed' }
  # bar: [[name, x, y], ...] in client coordinates of the main window (from the runner's ui map)
  $top = Clean $path[0]; $pos = $null
  foreach ($b in $bar) { if ((Clean $b[0]) -eq $top) { $pos = $b } }
  if (-not $pos) { throw "unknown menu bar entry: $top" }
  # up to 3 tries: after a failed try close menus/Flash dialogs, give the foreground back to the main window
  for ($try = 0; $try -lt 3; $try++) {
    if ($try) { CloseMenus | Out-Null; CloseFlash | Out-Null; [PR.N]::MainHwnd = (MainWin)
      $m = MainWin; [PR.N]::keybd_event(0x12, 0, 0, [IntPtr]::Zero); [PR.N]::keybd_event(0x12, 0, 2, [IntPtr]::Zero); [PR.N]::SetForegroundWindow($m) | Out-Null; Start-Sleep -Milliseconds 500 }
    $cr = CRect (MainWin)
    Click ($cr[0] + [int]$pos[1]) ($cr[1] + [int]$pos[2]) 'left' @() 1
    for ($t = 0; $t -lt 8 -and (MenuWins).Count -eq 0; $t++) { Start-Sleep -Milliseconds 150 }
    if ((MenuWins).Count) { break }
  }
  if ((MenuWins).Count -eq 0) { $f = [PR.N]::GetForegroundWindow()
    throw "menu did not open: $top (main enabled=$([PR.N]::IsWindowEnabled((MainWin))), foreground=$([PR.N]::Cls($f)), windows=$((@([PR.N]::Tops([uint32]$S.pid) | % { [PR.N]::Cls($_) })) -join ','))" }
  $invoked = $false; $seen = @()
  for ($k = 1; $k -lt $path.Count; $k++) {
    $want = Clean $path[$k]; $hit = $null
    for ($t = 0; $t -lt 10 -and -not $hit; $t++) {
      # newest (innermost) menu last: exact text first, then an abbreviated entry ("Gaußscher Weichz." for
      # "Gaußscher Weichzeichner") or a longer one that starts with the wanted text
      # menus opened by the previous step first (a sub menu entry wins over an equally named parent entry)
      $all = @((OpenMenus).menus); $menus = @($all | ? { $seen -notcontains $_.hwnd }) + @($all | ? { $seen -contains $_.hwnd })
      [array]::Reverse($menus)
      foreach ($mw in $menus) { foreach ($it in $mw.items) { if (-not $it.separator -and $it.text -eq $want) { $hit = $it } } }
      if (-not $hit) { foreach ($mw in $menus) { foreach ($it in $mw.items) { if ($it.separator -or -not $it.text) { continue }
        $a = $it.text.TrimEnd('.'); if (($it.text.EndsWith('.') -and $a.Length -ge 4 -and $want.StartsWith($a, [StringComparison]::OrdinalIgnoreCase)) -or ($want.Length -ge 4 -and $it.text.StartsWith($want, [StringComparison]::OrdinalIgnoreCase))) { $hit = $it } } } }
      if (-not $hit) { Start-Sleep -Milliseconds 150 }
    }
    if (-not $hit) { throw "menu item not found: $want" }
    $seen = @($all | % { $_.hwnd })
    $r = $hit.rect; $x = [int](($r[0] + $r[2]) / 2); $y = [int](($r[1] + $r[3]) / 2)
    $last = ($k -eq $path.Count - 1)
    # 'auto': a leaf entry at the end of the path is executed, a sub menu is left open
    if ($last -and ($final -eq 'invoke' -or ($final -eq 'auto' -and -not $hit.submenu))) {
      if ($hit.grayed) { throw "menu item disabled: $want" }
      # entries that would end the program (Datei > Beenden) are never executed
      $full = (@($path | % { Clean $_ })) -join ' > '
      if ($S.forbidden -contains $full) { CloseMenus | Out-Null; throw "forbidden menu entry (never executed in the reference program): $full" }
      # hover first (the item gets highlighted), then click; if the menu is still open afterwards the click
      # did not arrive: execute the highlighted item with Enter
      [PR.N]::SetCursorPos($x, $y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 250
      Click $x $y 'left' @() 1
      for ($t = 0; $t -lt 8 -and (MenuWins).Count; $t++) { Start-Sleep -Milliseconds 100 }
      if ((MenuWins).Count) { [PR.N]::SetCursorPos($x, $y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 200; Combo 'enter'; Start-Sleep -Milliseconds 300 }
      if ((MenuWins).Count) { throw "menu item did not execute (menu still open): $want" }
      $invoked = $true
    } else {
      [PR.N]::SetCursorPos($x, $y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 500
      if ($hit.submenu -and -not $last) { } elseif ($hit.submenu) { Start-Sleep -Milliseconds 200 }
    }
  }
  @{ menus = @(MenuWins | % { WRect $_ }); invoked = $invoked }
}
function Handle($c) {
  switch ($c.cmd) {
    'init' { $S.process = $c.process; $S.main_class = [string]$c.main_class; $S.view_class = [string]$c.view_class; $S.panel_class = [string]$c.panel_class; $S.forbidden = @($c.forbidden_menu | ? { $_ } | % { (($_ -split '>') | % { Clean $_ }) -join ' > ' }); [PR.N]::TargetClass = $S.main_class; $p = Proc; return @{ pid = $S.pid } }
    'ping' { return 'pong' }
    'pid' { Proc | Out-Null; return $S.pid }
    'main_pid' { return (MainOwner) }
    'procs' { return @(ProcList) }
    'windows' { if (-not (Proc)) { return @() }; return @([PR.N]::Tops([uint32]$S.pid) | % { WinInfo $_ }) }
    'main' { $h = MainWin; $i = WinInfo $h; $cr = CRect $h; $wr = $i.rect
             # the program's own frame reports a client area 1 px taller than the window: clip to the window
             $i.client = @([Math]::Max($cr[0], $wr[0]), [Math]::Max($cr[1], $wr[1]), [Math]::Min($cr[2], $wr[2]), [Math]::Min($cr[3], $wr[3])); $i.maximized = [PR.N]::IsZoomed($h); return $i }
    'place' { $h = MainWin; if ([PR.N]::IsZoomed($h) -or [PR.N]::IsIconic($h)) { [PR.N]::ShowWindow($h, 9) | Out-Null; Start-Sleep -Milliseconds 300 }
              [PR.N]::MoveWindow($h, [int]$c.x, [int]$c.y, [int]$c.w, [int]$c.h, $true) | Out-Null; Start-Sleep -Milliseconds 300; return (WRect $h) }
    'focus' { return (Focus) }
    'fg' { $f = [PR.N]::GetForegroundWindow(); return @{ hwnd = [int64]$f; class = [PR.N]::Cls($f); pid = [PR.N]::Pid($f); title = [PR.N]::Txt($f); guard = [PR.N]::Guard() } }
    'view' { $m = MainWin; $best = $null; $area = 0
             foreach ($h in [PR.N]::Kids($m)) { if ([PR.N]::Cls($h) -eq $S.view_class) { $r = CRect $h; $a = ($r[2] - $r[0]) * ($r[3] - $r[1]); if ($a -gt $area) { $area = $a; $best = $r } } }
             if (-not $best) { throw 'view window not found' }; return $best }
    'panel' { $m = MainWin; $cands = @([PR.N]::Kids($m)); foreach ($t in [PR.N]::Tops([uint32]$S.pid)) { if ($t -ne $m) { $cands += $t; $cands += @([PR.N]::Kids($t)) } }
              foreach ($h in $cands) { if ((Clean ([PR.N]::Txt($h))) -eq (Clean $c.name) -and (-not $S.panel_class -or [PR.N]::Cls($h) -eq $S.panel_class)) {
                  $p = [PR.N]::GetParent($h); $g = if ($p -ne [IntPtr]::Zero -and $p -ne $m) { WRect $p } else { WRect $h }
                  return @{ rect = $g; palette = (WRect $h); hwnd = [int64]$h } } }
              throw "panel not visible: $($c.name)" }
    'find_child' {
      $m = MainWin; $roots = @()
      if ($c.scope -eq 'dialog') { $roots = @(Dialogs) } elseif ($c.scope -and $c.scope -ne 'main') { $roots = @([IntPtr][int64](Handle @{ cmd = 'panel'; name = $c.scope }).hwnd) } else { $roots = @($m) + @([PR.N]::Tops([uint32]$S.pid) | ? { $_ -ne $m }) }
      $n = 0
      foreach ($r in $roots) { foreach ($h in [PR.N]::Kids($r)) {
        if ($c.class -and [PR.N]::Cls($h) -ne $c.class) { continue }
        if ($c.title -and (Clean ([PR.N]::Txt($h))) -ne (Clean $c.title)) { continue }
        if ($n -eq [int]$c.index) { $i = WinInfo $h; $i.title = [PR.N]::Txt($h); return $i }; $n++ } }
      throw "child not found: $($c | ConvertTo-Json -Compress)" }
    'set_text' { [PR.N]::SendMessageS([IntPtr][int64]$c.hwnd, 0x000C, [IntPtr]::Zero, [string]$c.text) | Out-Null; return 'ok' }
    'next_ctl_root' { return [int64][PR.N]::GetAncestor([IntPtr][int64]$c.hwnd, 2) }
    'next_ctl' { # Tab / Shift+Tab inside a dialog: the program's dialogs do not translate Tab (SendInput Tab types a
      # character into the edit), WM_NEXTDLGCTL moves the focus like the dialog manager does (the edit commits on focus loss)
      $h = [PR.N]::GetAncestor([IntPtr][int64]$c.hwnd, 2)
      if ($h -eq [IntPtr]::Zero -or [PR.N]::Cls($h) -ne '#32770') { throw 'next_ctl: control is not inside a dialog' }
      [PR.N]::SendMessage($h, 0x0028, [IntPtr]$(if ($c.back) { 1 } else { 0 }), [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 120
      $f = [PR.N]::FocusOf($h); return @{ dialog = [int64]$h; focus = [int64]$f; focus_class = $(if ($f -ne [IntPtr]::Zero) { [PR.N]::Cls($f) } else { '' }) } }
    'get_text' { $sb = New-Object Text.StringBuilder 1024; [PR.N]::SendMessageSB([IntPtr][int64]$c.hwnd, 0x000D, [IntPtr]1024, $sb) | Out-Null; return $sb.ToString() }
    'combo_items' { $h = [IntPtr][int64]$c.hwnd; $n = [int][PR.N]::SendMessage($h, 0x146, [IntPtr]::Zero, [IntPtr]::Zero); $l = @()
      for ($i = 0; $i -lt $n; $i++) { $sb = New-Object Text.StringBuilder 512; [PR.N]::SendMessageSB($h, 0x148, [IntPtr]$i, $sb) | Out-Null; $l += $sb.ToString() }
      return @{ items = $l; selected = (I32 ([PR.N]::SendMessage($h, 0x147, [IntPtr]::Zero, [IntPtr]::Zero))) } }
    'combo_select' { $h = [IntPtr][int64]$c.hwnd; $i = I32 ([PR.N]::SendMessageS($h, 0x158, [IntPtr](-1), [string]$c.text))
      if ($i -lt 0) { throw "combo entry not found: $($c.text)" }
      [PR.N]::SendMessage($h, 0x14E, [IntPtr]$i, [IntPtr]::Zero) | Out-Null
      $id = [PR.N]::GetDlgCtrlID($h); [PR.N]::SendMessage([PR.N]::GetParent($h), 0x111, [IntPtr]((1 -shl 16) -bor ($id -band 0xFFFF)), $h) | Out-Null
      return $i }
    'children' { $m = MainWin; return @([PR.N]::Kids($m) | % { WinInfo $_ } | ? { $_.title -or $c.all }) }
    'dialogs' { return @(Dialogs | % { $i = WinInfo $_; $i.texts = DialogTexts $_; $i }) }
    'dialog_close' { $n = 0; $cmdId = if ($c.mode -eq 'ok') { 1 } else { 2 }
      foreach ($h in @(Dialogs)) { $t = "$([PR.N]::Txt($h)) | $(DialogTexts $h)"; if ($c.except -and $t -match $c.except) { continue }
        # decline: "No" first (save / recovery questions: do not save, do not recover), then Cancel/Close;
        # never Yes/Save/OK
        $lists = if ($c.mode -eq 'ok') { @(, @('OK')) } else { @(@('Nein', 'No', 'Nicht speichern', "Don't Save"), @('Abbrechen', 'Cancel', 'Schließen', 'Close')) }
        if ([PR.N]::Cls($h) -eq $FLASHDLG) { if ($c.mode -ne 'ok') { [PR.N]::PostMessage($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null; $n++ }; continue }
        $kids = @([PR.N]::Kids($h) | ? { [PR.N]::Cls($_) -eq 'Button' }); $btn = $null
        foreach ($names in $lists) { if (-not $btn) { $btn = $kids | ? { $names -contains (Clean ([PR.N]::Txt($_))) } | Select-Object -First 1 } }
        if ($btn) { [PR.N]::PostMessage($btn, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null } else { [PR.N]::PostMessage($h, 0x111, [IntPtr]$cmdId, [IntPtr]::Zero) | Out-Null }; $n++ }
      # property-sheet style dialogs (e.g. the brush stroke editor) ignore a posted click on their Cancel
      # button: IDCANCEL to the dialog, then the system close command
      if ($n -and $c.mode -ne 'ok') { Start-Sleep -Milliseconds 400
        foreach ($h in @(Dialogs)) { $t = "$([PR.N]::Txt($h)) | $(DialogTexts $h)"; if ($c.except -and $t -match $c.except) { continue }
          if ([PR.N]::Cls($h) -eq $FLASHDLG) { continue }
          [PR.N]::PostMessage($h, 0x111, [IntPtr]2, [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 300
          if ([PR.N]::IsWindowVisible($h)) { [PR.N]::PostMessage($h, 0x0112, [IntPtr]0xF060, [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 300 } } }
      return $n }
    'focused' { $f = [PR.N]::GetForegroundWindow(); if ($f -eq [IntPtr]::Zero -or [PR.N]::Pid($f) -ne [uint32]$S.pid) { $f = MainWin }
                $h = [PR.N]::FocusOf($f); if ($h -eq [IntPtr]::Zero) { return $null }
                $i = WinInfo $h; $p = [PR.N]::GetParent($h); if ($p -ne [IntPtr]::Zero) { $i.parent = @{ class = [PR.N]::Cls($p); title = [PR.N]::Txt($p) } }; return $i }
    'close_window' { $h = [IntPtr][int64]$c.hwnd; if ([PR.N]::Pid($h) -ne [uint32]$S.pid -or $h -eq (MainWin)) { throw 'not a tool window of the target' }
                     [PR.N]::PostMessage($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 300; return [PR.N]::IsWindowVisible($h) }
    'menus' { return (OpenMenus) }
    'kids' { return @([PR.N]::Kids([IntPtr][int64]$c.hwnd) | % { WinInfo $_ }) }
    'main_enabled' { return [PR.N]::IsWindowEnabled((MainWin)) }
    'close_menus' { return (CloseMenus) }
    'menu' { return (MenuPath @($c.path) $c.final @($c.bar)) }
    'uia_tree' { $root = if ($c.hwnd) { El ([IntPtr][int64]$c.hwnd) } else { El (MainWin) }; $out = New-Object Collections.ArrayList; Dump $root 0 ([int]$(if ($c.depth) { $c.depth } else { 3 })) $out; return ,$out.ToArray() }
    'uia_find' { $e = FindEl $c.name $c.type $c.scope $c.exact; if (-not $e) { throw "not found: $($c.name) [$($c.type)]" }; return (ElInfo $e) }
    'uia_do' { $e = FindEl $c.name $c.type $c.scope $c.exact; if (-not $e) { throw "not found: $($c.name) [$($c.type)]" }; $r = DoAct $e $c.action $c.value; $i = ElInfo $e; $i.done = $r; return $i }
    'key' { Combo $c.combo; Start-Sleep -Milliseconds 80; return 'ok' }
    'type' { foreach ($ch in ([string]$c.text).ToCharArray()) { [PR.N]::Send(@([PR.N]::Uni($ch, $false), [PR.N]::Uni($ch, $true))); Start-Sleep -Milliseconds 25 }; return 'ok' }
    'click' { Click $c.x $c.y $c.button $c.mods $c.count; return 'ok' }
    'move' { [PR.N]::SetCursorPos([int]$c.x, [int]$c.y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 120; return 'ok' }
    'drag' { Drag $c.x1 $c.y1 $c.x2 $c.y2 $c.mods $c.steps $c.button; return 'ok' }
    'wheel' { [PR.N]::SetCursorPos([int]$c.x, [int]$c.y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 60; Mods $c.mods $false
              $fl = if ($c.horizontal) { 0x1000 } else { 0x800 }; $n = [int]$c.delta
              for ($i = 0; $i -lt [Math]::Abs($n); $i++) { [PR.N]::Send(@([PR.N]::Mouse($fl, [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]([Math]::Sign($n) * 120)), 0)))); Start-Sleep -Milliseconds 60 }
              Mods $c.mods $true; Start-Sleep -Milliseconds 150; return 'ok' }
    'down' { [PR.N]::SetCursorPos([int]$c.x, [int]$c.y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 60; Mods $c.mods $false
             $d = if ($c.button -eq 'right') { 0x8 } else { 0x2 }; [PR.N]::Send(@([PR.N]::Mouse($d, 0))); Start-Sleep -Milliseconds 80; return 'ok' }
    'held_move' { $x0 = [int]$c.x1; $y0 = [int]$c.y1; $st = [Math]::Max(1, [int]$c.steps)
                  for ($i = 1; $i -le $st; $i++) { [PR.N]::SetCursorPos([int]($x0 + ($c.x2 - $x0) * $i / $st), [int]($y0 + ($c.y2 - $y0) * $i / $st)) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 25 }; return 'ok' }
    'up' { if ($c.x -ne $null) { [PR.N]::SetCursorPos([int]$c.x, [int]$c.y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 40 }
           $u = if ($c.button -eq 'right') { 0x10 } else { 0x4 }; [PR.N]::Send(@([PR.N]::Mouse($u, 0))); Start-Sleep -Milliseconds 60; Mods $c.mods $true; return 'ok' }
    'menu_tree' { $out = New-Object Collections.ArrayList; foreach ($w in (MenuWins)) { $m = [PR.N]::SendMessage($w, 0x01E1, [IntPtr]::Zero, [IntPtr]::Zero); if ($m -ne [IntPtr]::Zero) { TreeOf $m '' $out } }; return ,$out.ToArray() }
    'at' { $pt = New-Object PR.N+POINT; $pt.X = [int]$c.x; $pt.Y = [int]$c.y; $h = [PR.N]::WindowFromPoint($pt); if ($h -eq [IntPtr]::Zero) { return $null }
           $i = WinInfo $h; $i.title = [PR.N]::Txt($h); $p = [PR.N]::GetParent($h); if ($p -ne [IntPtr]::Zero) { $i.parent = WinInfo $p }; $i.pid = [PR.N]::Pid($h); return $i }
    # smallest visible Edit/ComboBox child of the target containing the point (WindowFromPoint may hit a tooltip
    # or a transparent overlay instead)
    'field_at' { $best = $null; $area = [int64]::MaxValue; $m = MainWin
      foreach ($top in @($m) + @([PR.N]::Tops([uint32]$S.pid) | ? { $_ -ne $m })) { foreach ($k in [PR.N]::Kids($top)) {
        $cl = [PR.N]::Cls($k); if ($cl -ne 'Edit' -and $cl -ne 'ComboBox') { continue }
        $r = WRect $k; if ($c.x -lt $r[0] -or $c.x -ge $r[2] -or $c.y -lt $r[1] -or $c.y -ge $r[3]) { continue }
        $a = [int64]($r[2] - $r[0]) * ($r[3] - $r[1]); if ($a -lt $area) { $area = $a; $best = $k } } }
      if (-not $best) { return $null }; $i = WinInfo $best; $p = [PR.N]::GetParent($best); if ($p -ne [IntPtr]::Zero) { $i.parent = WinInfo $p }; return $i }
    'park' { [PR.N]::SetCursorPos([int]$c.x, [int]$c.y) | Out-Null; return 'ok' }
    'shot' { $r = @($c.rect); $b = [PR.N]::Screen($r[0], $r[1], $r[2], $r[3]); $b.Save($c.path, [System.Drawing.Imaging.ImageFormat]::Png); $b.Dispose(); return $c.path }
    default { throw "unknown command $($c.cmd)" }
  }
}

$PIPE_IN = New-Object IO.StreamReader([Console]::OpenStandardInput(), (New-Object Text.UTF8Encoding $false))
$PIPE_OUT = [Console]::OpenStandardOutput()
function Emit($o) { $b = [Text.Encoding]::UTF8.GetBytes(($o | ConvertTo-Json -Compress -Depth 8) + "`n"); $PIPE_OUT.Write($b, 0, $b.Length); $PIPE_OUT.Flush() }
while ($true) {
  $line = $PIPE_IN.ReadLine()
  if ($null -eq $line) { break }
  if (-not $line.Trim()) { continue }
  $id = 0
  try {
    $c = $line | ConvertFrom-Json
    $id = $c.id
    if ($c.cmd -eq 'quit') { Emit @{ id = $id; ok = $true; result = 'bye' }; break }
    $res = Handle $c
    $out = @{ id = $id; ok = $true; result = $res }
  } catch {
    $out = @{ id = $id; ok = $false; error = "$($_.Exception.Message)" }
  }
  Emit $out
}
