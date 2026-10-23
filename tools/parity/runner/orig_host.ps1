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
  [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
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
  // R-5: foreground taken back with the AttachThreadInput trick, a neutral caption click as last resort
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  // R-7 (run 11, Q-43): second way when SetForegroundWindow is not honoured or the focus is taken again at once
  [DllImport("user32.dll")] public static extern void SwitchToThisWindow(IntPtr h, bool alt);
  // R-8 (run 12): the host may pass its foreground right on to the target process while attached
  [DllImport("user32.dll")] public static extern bool AllowSetForegroundWindow(uint pid);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f);
  // R-6 (Q-39): process name and title of a window for the protocol (foreign foreground windows)
  public static string ProcName(IntPtr h) { try { return System.Diagnostics.Process.GetProcessById((int)Pid(h)).ProcessName; } catch { return "?"; } }
  public static string Who(IntPtr h) { string t = Txt(h); if (t.Length > 60) t = t.Substring(0, 60); return ProcName(h) + " '" + t + "'"; }
  public static uint Tid(IntPtr h) { uint p; return GetWindowThreadProcessId(h, out p); }
  public static int HitTest(IntPtr h, int x, int y) { return (int)SendMessage(h, 0x84, IntPtr.Zero, (IntPtr)(((y & 0xFFFF) << 16) | (x & 0xFFFF))); }
  public static void SendRaw(INPUT[] a) { SendInput((uint)a.Length, a, Marshal.SizeOf(typeof(INPUT))); }
  // R-5: wait until the target's UI thread takes messages again (WM_NULL with timeout); returns the ms waited.
  // A slow program (moving the 2400 x 1600 bitmap of T10) otherwise gets the moves of a drag coalesced into one
  [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeout(IntPtr h, uint m, IntPtr w, IntPtr l, uint f, uint t, out IntPtr r);
  public static int Idle(int ms) { if (MainHwnd == IntPtr.Zero) return 0; var sw = System.Diagnostics.Stopwatch.StartNew(); IntPtr r; SendMessageTimeout(MainHwnd, 0, IntPtr.Zero, IntPtr.Zero, 2, (uint)ms, out r); return (int)sw.ElapsedMilliseconds; }
  public static string Cls(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
  public static string Txt(IntPtr h) { var s = new StringBuilder(512); GetWindowText(h, s, 512); return s.ToString(); }
  public static uint Pid(IntPtr h) { uint p; GetWindowThreadProcessId(h, out p); return p; }
  public static List<IntPtr> Tops(uint pid) { var l = new List<IntPtr>(); EnumWindows((h, x) => { if (IsWindowVisible(h) && Pid(h) == pid) l.Add(h); return true; }, IntPtr.Zero); return l; }
  public static List<IntPtr> Kids(IntPtr p) { var l = new List<IntPtr>(); EnumChildWindows(p, (h, x) => { if (IsWindowVisible(h)) l.Add(h); return true; }, IntPtr.Zero); return l; }
  public static uint TargetPid = 0; public static string TargetClass = ""; public static IntPtr MainHwnd = IntPtr.Zero;
  public static string Guard() { if (TargetPid == 0) return "no target"; IntPtr f = GetForegroundWindow(); if (f == IntPtr.Zero) return "no foreground window";
    if (Pid(f) != TargetPid) return "foreground belongs to process " + Pid(f) + " (" + Cls(f) + ") " + Who(f);
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
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint mapType);
  [DllImport("user32.dll")] public static extern uint MapVirtualKeyEx(uint code, uint mapType, IntPtr hkl);
  [DllImport("user32.dll")] public static extern IntPtr GetKeyboardLayout(uint tid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr LoadKeyboardLayout(string klid, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr ActivateKeyboardLayout(IntPtr hkl, uint flags);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetKeyNameText(int l, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int ToUnicodeEx(uint vk, uint scan, byte[] state, StringBuilder s, int n, uint flags, IntPtr hkl);
  [DllImport("user32.dll")] public static extern IntPtr SetFocus(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsChild(IntPtr p, IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetDoubleClickTime();
  [DllImport("user32.dll")] public static extern int GetSystemMetrics(int i);
  // R-13b: keys carry their scan code (Escape = 0x01).
  // R-15 (TW-140, run 19: Delete selected the Eraser, Right the Text tool, Escape gave a "c"/the Crop tool): the
  // extended flags were already right. Most likely (it explains all three cases) the reference matches plain keys by the key's NAME (GetKeyNameText of scan
  // code + extended bit in the target thread's keyboard layout); when the names do not match its shortcut set
  // ("Delete" vs "Entf") it falls back to a tool letter - the name's last letter: Delet-e = Eraser, Righ-t = Text,
  // Es-c = Crop. Scan codes now come from the TARGET thread's layout (KeyHkl, set by `layout`/`key_echo`; letters
  // differ between layouts, e.g. Y/Z), extended keys from an explicit table (Insert, Delete, Home, End, PgUp, PgDn,
  // arrows, Win, Apps, NumPad /, NumLock, right Ctrl/Alt), Escape always 0x01 without the extended flag.
  public static IntPtr KeyHkl = IntPtr.Zero;
  public static bool IsExtended(ushort vk) { return (vk >= 0x21 && vk <= 0x28) || vk == 0x2D || vk == 0x2E || vk == 0x5B || vk == 0x5C || vk == 0x5D || vk == 0x6F || vk == 0x90 || vk == 0xA3 || vk == 0xA5; }
  public static ushort ScanOf(ushort vk) { if (vk == 0x1B) return 0x01; uint s = KeyHkl != IntPtr.Zero ? MapVirtualKeyEx(vk, 0, KeyHkl) : MapVirtualKey(vk, 0); return (ushort)(s & 0xFF); }
  public static INPUT Key(ushort vk, bool up) { var i = new INPUT(); i.type = 1; i.u.ki.wVk = vk; i.u.ki.wScan = ScanOf(vk); i.u.ki.dwFlags = up ? 2u : 0u;
    if (IsExtended(vk)) i.u.ki.dwFlags |= 1; return i; }
  // R-27 (TW-179, RE-24: the vk + scan code path turned a plain Escape into the Eraser, Enter into a tool switch; a
  // posted WM_KEYDOWN had no effect; the scan code alone arrived right - T = Text tool, Escape removes a crop box,
  // Enter crops): scan-code-only input (KEYEVENTF_SCANCODE, wVk = 0, extended flag for extended keys) - the system
  // derives the virtual key from the scan code like for a real keyboard. Null when the key has no scan code.
  public static INPUT? KeySc(ushort vk, bool up) { ushort s = ScanOf(vk); if (s == 0) return null; var i = new INPUT(); i.type = 1; i.u.ki.wVk = 0; i.u.ki.wScan = s;
    i.u.ki.dwFlags = 8u | (up ? 2u : 0u); if (IsExtended(vk)) i.u.ki.dwFlags |= 1; return i; }
  // key name / character as the target sees them (layout hkl; the host thread borrows that layout for the call)
  public static string KeyName(uint scan, bool ext, IntPtr hkl) { IntPtr old = IntPtr.Zero; if (hkl != IntPtr.Zero) old = ActivateKeyboardLayout(hkl, 0);
    var s = new StringBuilder(64); GetKeyNameText((int)((scan << 16) | (ext ? (1u << 24) : 0u)), s, 64); if (old != IntPtr.Zero) ActivateKeyboardLayout(old, 0); return s.ToString(); }
  public static string KeyChar(uint vk, uint scan, IntPtr hkl) { var s = new StringBuilder(8); int n = ToUnicodeEx(vk, scan, new byte[256], s, 8, 4, hkl); return n > 0 ? s.ToString(0, n) : ""; }
  // R-20 (TW-146): typed text as real keys - virtual key and shift state of the character in the target's layout
  // (VkKeyScanEx with KeyHkl), every key with its scan code (Key); null = no key for it in that layout (sent as Unicode)
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern short VkKeyScanEx(char ch, IntPtr hkl);
  public static INPUT[] CharKeys(char c) { return CharKeys(c, false); }
  // R-27 (TW-179): sc = every key of the character (Shift/Ctrl/Alt too) as scan-code-only input (KeySc)
  public static INPUT[] CharKeys(char c, bool sc) { IntPtr hkl = KeyHkl != IntPtr.Zero ? KeyHkl : GetKeyboardLayout(0); short r = VkKeyScanEx(c, hkl);
    if (r == -1 || (r & 0xFF) == 0xFF) return null; ushort vk = (ushort)(r & 0xFF); int sh = (r >> 8) & 0xFF; var a = new List<INPUT>();
    if (sc && ScanOf(vk) == 0) return null;
    Func<ushort, bool, INPUT> K = (v, up) => sc ? KeySc(v, up).Value : Key(v, up);
    if ((sh & 2) != 0) a.Add(K(0x11, false)); if ((sh & 4) != 0) a.Add(K(0x12, false)); if ((sh & 1) != 0) a.Add(K(0x10, false));
    a.Add(K(vk, false)); a.Add(K(vk, true));
    if ((sh & 1) != 0) a.Add(K(0x10, true)); if ((sh & 4) != 0) a.Add(K(0x12, true)); if ((sh & 2) != 0) a.Add(K(0x11, true)); return a.ToArray(); }
  // R-20 (TW-146): GUI thread state of a window's thread (focus, caret window, caret rectangle, flags 1 = caret blinking)
  public static GUITHREADINFO GuiOf(IntPtr top) { uint p; uint tid = GetWindowThreadProcessId(top, out p); var g = new GUITHREADINFO(); g.cbSize = Marshal.SizeOf(typeof(GUITHREADINFO)); GetGUIThreadInfo(tid, ref g); return g; }
  public static INPUT Uni(char c, bool up) { var i = new INPUT(); i.type = 1; i.u.ki.wScan = c; i.u.ki.dwFlags = 4u | (up ? 2u : 0u); return i; }
  public static INPUT Mouse(uint flags, uint data) { var i = new INPUT(); i.type = 0; i.u.mi.dwFlags = flags; i.u.mi.mouseData = data; return i; }
  public static Bitmap Screen(int l, int t, int r, int b) { var bmp = new Bitmap(Math.Max(1, r - l), Math.Max(1, b - t)); using (var g = Graphics.FromImage(bmp)) g.CopyFromScreen(l, t, 0, 0, bmp.Size); return bmp; }
}
// R-15 (TW-140): `key_echo` - a low-level keyboard hook on its own thread records every key event as the system
// delivers it (message, vk, scan, flags: 1 = extended, 0x10 = injected) and swallows it, so the self test types
// nothing into any program
public static class E {
  public delegate IntPtr LLProc(int code, IntPtr w, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct KLL { public uint vk, scan, flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] public struct EMSG { public IntPtr hwnd; public uint message; public IntPtr wParam, lParam; public uint time; public int x, y; }
  [DllImport("user32.dll", SetLastError=true)] static extern IntPtr SetWindowsHookEx(int id, LLProc p, IntPtr mod, uint tid);
  [DllImport("user32.dll")] static extern bool UnhookWindowsHookEx(IntPtr h);
  [DllImport("user32.dll")] static extern IntPtr CallNextHookEx(IntPtr h, int c, IntPtr w, IntPtr l);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode)] static extern IntPtr GetModuleHandle(string n);
  [DllImport("user32.dll")] static extern int GetMessage(out EMSG m, IntPtr h, uint a, uint b);
  [DllImport("user32.dll")] static extern bool PostThreadMessage(uint tid, uint m, IntPtr w, IntPtr l);
  [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
  public static List<uint[]> Got = new List<uint[]>();
  static LLProc P; static IntPtr H = IntPtr.Zero; static uint T; static System.Threading.Thread Th; static string Err = "";
  static IntPtr Proc(int c, IntPtr w, IntPtr l) {
    if (c >= 0) { var k = (KLL)Marshal.PtrToStructure(l, typeof(KLL)); lock (Got) { Got.Add(new uint[] { (uint)w.ToInt64(), k.vk, k.scan, k.flags }); } return (IntPtr)1; }
    return CallNextHookEx(H, c, w, l); }
  static void Loop(object o) { var ok = (System.Threading.ManualResetEvent)o; T = GetCurrentThreadId();
    H = SetWindowsHookEx(13, P, GetModuleHandle(null), 0); if (H == IntPtr.Zero) Err = "SetWindowsHookEx error " + Marshal.GetLastWin32Error();
    ok.Set(); if (H == IntPtr.Zero) return; EMSG m; while (GetMessage(out m, IntPtr.Zero, 0, 0) > 0) { } UnhookWindowsHookEx(H); H = IntPtr.Zero; }
  public static string Start() { Stop(); lock (Got) { Got.Clear(); } Err = ""; P = Proc; var ok = new System.Threading.ManualResetEvent(false);
    Th = new System.Threading.Thread(Loop); Th.IsBackground = true; Th.Start(ok); if (!ok.WaitOne(3000)) return "hook thread did not start"; return Err; }
  public static void Stop() { if (Th != null) { PostThreadMessage(T, 0x12, IntPtr.Zero, IntPtr.Zero); Th.Join(2000); Th = null; } }
  public static uint[][] Take() { lock (Got) { return Got.ToArray(); } }
}}
'@
[PR.N]::SetProcessDPIAware() | Out-Null
$UIA = [System.Windows.Automation.AutomationElement]
$TW = [System.Windows.Automation.TreeWalker]::ControlViewWalker
$S = @{ process = ''; main_class = ''; view_class = ''; panel_class = ''; pid = 0; forbidden = @(); plain_keys = 'scancode' }

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

function Focus([bool]$hard) {
  $h = MainWin
  [PR.N]::TargetClass = $S.main_class; [PR.N]::MainHwnd = $h
  if ([PR.N]::IsIconic($h)) { [PR.N]::ShowWindow($h, 9) | Out-Null }
  # a dialog of the target may legitimately own the foreground
  $f = [PR.N]::GetForegroundWindow()
  if ($f -ne [IntPtr]::Zero -and [PR.N]::Pid($f) -eq [uint32]$S.pid) { return @{ fg = [int64]$f; class = [PR.N]::Cls($f) } }
  $was = [PR.N]::Cls($f); $how = 'set'
  $wasPid = [PR.N]::Pid($f); $wasProc = [PR.N]::ProcName($f); $wasTitle = [PR.N]::Txt($f)
  # R-5 (run 9): the taskbar (Shell_TrayWnd) and a browser took the focus back within a second. A taskbar
  # flyout / start menu is closed with Esc first (the key goes to the shell, never to the target)
  if ($was -match '^(Shell_TrayWnd|Shell_SecondaryTrayWnd|NotifyIconOverflowWindow|Windows\.UI\.Core\.CoreWindow|XamlExplorerHostIslandWindow|TopLevelWindowForOverflowXamlIsland)$') {
    [PR.N]::keybd_event(0x1B, 0, 0, [IntPtr]::Zero); [PR.N]::keybd_event(0x1B, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 120; $how = 'esc+set' }
  # AttachThreadInput trick: share the input state with the foreground thread so SetForegroundWindow is honoured
  $me = [PR.N]::GetCurrentThreadId(); $ft = if ($f -ne [IntPtr]::Zero) { [PR.N]::Tid($f) } else { 0 }; $tt = [PR.N]::Tid($h)
  if ($ft -and $ft -ne $me) { [PR.N]::AttachThreadInput($me, $ft, $true) | Out-Null }
  if ($tt -and $tt -ne $me) { [PR.N]::AttachThreadInput($me, $tt, $true) | Out-Null }
  try {
    [PR.N]::AllowSetForegroundWindow([uint32]$S.pid) | Out-Null
    [PR.N]::keybd_event(0x12, 0, 0, [IntPtr]::Zero); [PR.N]::keybd_event(0x12, 0, 2, [IntPtr]::Zero)
    [PR.N]::BringWindowToTop($h) | Out-Null; [PR.N]::SetForegroundWindow($h) | Out-Null
  } finally {
    if ($ft -and $ft -ne $me) { [PR.N]::AttachThreadInput($me, $ft, $false) | Out-Null }
    if ($tt -and $tt -ne $me) { [PR.N]::AttachThreadInput($me, $tt, $false) | Out-Null }
  }
  Start-Sleep -Milliseconds 250
  $f = [PR.N]::GetForegroundWindow()
  # R-7 (run 11: the take-back reported 'set', the next input was refused - another program took the foreground
  # straight back): SwitchToThisWindow when SetForegroundWindow was not honoured, then a stability check (the
  # foreground must still be ours 400 ms later; up to two more rounds with the Alt tap + AttachThreadInput trick)
  if ([PR.N]::Pid($f) -ne [uint32]$S.pid) {
    [PR.N]::keybd_event(0x12, 0, 0, [IntPtr]::Zero); [PR.N]::keybd_event(0x12, 0, 2, [IntPtr]::Zero)
    [PR.N]::SwitchToThisWindow($h, $true); $how += '+switch'; Start-Sleep -Milliseconds 250
    $f = [PR.N]::GetForegroundWindow() }
  foreach ($round in 1, 2) {
    if ([PR.N]::Pid($f) -ne [uint32]$S.pid) { break }
    Start-Sleep -Milliseconds 400
    $f = [PR.N]::GetForegroundWindow()
    if ([PR.N]::Pid($f) -eq [uint32]$S.pid) { $how += '+stable'; break }
    $how += "+lost to $([PR.N]::ProcName($f))"
    $ft = [PR.N]::Tid($f)
    if ($ft -and $ft -ne $me) { [PR.N]::AttachThreadInput($me, $ft, $true) | Out-Null }
    try {
      [PR.N]::keybd_event(0x12, 0, 0, [IntPtr]::Zero); [PR.N]::keybd_event(0x12, 0, 2, [IntPtr]::Zero)
      [PR.N]::SetForegroundWindow($h) | Out-Null; [PR.N]::SwitchToThisWindow($h, $true)
    } finally { if ($ft -and $ft -ne $me) { [PR.N]::AttachThreadInput($me, $ft, $false) | Out-Null } }
    Start-Sleep -Milliseconds 250
    $f = [PR.N]::GetForegroundWindow() }
  $covered = @()
  if ($hard -and [PR.N]::Pid($f) -ne [uint32]$S.pid) {
    # last resort: one click on a neutral spot of the target's caption (WM_NCHITTEST = HTCAPTION). The topmost
    # enabled window of the target is used (a modal dialog before the main window); the point must belong to that
    # window (WindowFromPoint), otherwise no click is sent.
    # R-6 (Q-39, run 10): the click never fired - the foreign window covered the caption, the check skipped every
    # point silently. Now: fresh window rect, the middle of the caption (vertical centre of the HTCAPTION band,
    # x at 50/35/65 %), the covering window is reported; when covered, the target is raised for the click
    # (HWND_TOPMOST without activation, reset to HWND_NOTOPMOST right after) and the point checked again. After
    # the click the foreground is checked and reported (`+caption x,y ok|nofocus`).
    $wins = @([PR.N]::Tops([uint32]$S.pid) | ? { [PR.N]::IsWindowEnabled($_) -and [PR.N]::Cls($_) -ne '#32768' })
    if ([PR.N]::IsWindowEnabled($h)) { $wins = @($wins | ? { $_ -ne $h -and [PR.N]::Cls($_) -eq '#32770' }) + @($h) }
    $clicked = $false
    foreach ($w in $wins) {
      $raised = $false
      try {
        foreach ($pass in 0, 1) {
          $r = WRect $w; $pt = $null
          foreach ($fx in 0.5, 0.35, 0.65) { $x = [int]($r[0] + ($r[2] - $r[0]) * $fx); $ys = @()
            for ($y = $r[1] + 1; $y -lt [Math]::Min($r[3], $r[1] + 60); $y += 2) { if ([PR.N]::HitTest($w, $x, $y) -eq 2) { $ys += $y } elseif ($ys.Count) { break } }
            if (-not $ys.Count) { continue }
            $y = $ys[[int][Math]::Floor(($ys.Count - 1) / 2)]
            $p = New-Object PR.N+POINT; $p.X = $x; $p.Y = $y; $at = [PR.N]::WindowFromPoint($p)
            $root = if ($at -ne [IntPtr]::Zero) { [PR.N]::GetAncestor($at, 2) } else { [IntPtr]::Zero }
            if ($root -eq $w) { $pt = @($x, $y); break }
            if ($root -ne [IntPtr]::Zero) { $covered += "$x,$y by $([PR.N]::Cls($root)) pid $([PR.N]::Pid($root)) $([PR.N]::Who($root))" } }
          if ($pt -or $pass -eq 1) { break }
          # covered: raise the target above the foreign window without activating it (no focus change by itself)
          [PR.N]::SetWindowPos($w, [IntPtr](-1), 0, 0, 0, 0, 0x13) | Out-Null; $raised = $true; $how += '+raise'; Start-Sleep -Milliseconds 150 }
        if (-not $pt) { continue }
        [PR.N]::SetCursorPos($pt[0], $pt[1]) | Out-Null; Start-Sleep -Milliseconds 40
        [PR.N]::SendRaw(@([PR.N]::Mouse(0x2, 0))); Start-Sleep -Milliseconds 60; [PR.N]::SendRaw(@([PR.N]::Mouse(0x4, 0)))
        Start-Sleep -Milliseconds 300; $clicked = $true
        $f2 = [PR.N]::GetForegroundWindow()
        $how += "+caption $($pt[0]),$($pt[1]) $(if ([PR.N]::Pid($f2) -eq [uint32]$S.pid) { 'ok' } else { 'nofocus' })"
      } finally {
        if ($raised) { [PR.N]::SetWindowPos($w, [IntPtr](-2), 0, 0, 0, 0, 0x13) | Out-Null }
      }
      if ($clicked) { break } }
    if (-not $clicked) { $how += '+nocaption' }
    $f = [PR.N]::GetForegroundWindow()
  }
  # R-8 (run 12: 75 take-backs from claude/Chrome_WidgetWin_1 reported 'set+stable -> ok', the next input ~0.2 s
  # later was refused - the other program re-activates itself within a second): a final settle check, the
  # foreground must still be ours after $S.settle_ms (default 450 ms); otherwise `+relost to <proc> <class>` and ok=false
  if ([PR.N]::Pid($f) -eq [uint32]$S.pid) {
    $ms = 450; if ($S.settle_ms) { $ms = [int]$S.settle_ms }
    Start-Sleep -Milliseconds $ms
    $f = [PR.N]::GetForegroundWindow()
    if ([PR.N]::Pid($f) -eq [uint32]$S.pid) { $how += "+settled $ms" } else { $how += "+relost to $([PR.N]::ProcName($f)) $([PR.N]::Cls($f)) after $ms ms" } }
  @{ fg = [int64]$f; class = [PR.N]::Cls($f); ok = ([PR.N]::Pid($f) -eq [uint32]$S.pid); was = $was; how = $how
     was_pid = $wasPid; was_proc = $wasProc; was_title = $wasTitle; covered = @($covered | Select-Object -First 4) }
}

$VK = @{ shift = 0x10; ctrl = 0x11; alt = 0x12; space = 0x20; enter = 0x0D; return = 0x0D; esc = 0x1B; escape = 0x1B; tab = 0x09; backspace = 0x08; back = 0x08; delete = 0x2E; del = 0x2E; insert = 0x2D
  left = 0x25; up = 0x26; right = 0x27; down = 0x28; arrowleft = 0x25; arrowup = 0x26; arrowright = 0x27; arrowdown = 0x28; home = 0x24; end = 0x23; pageup = 0x21; pagedown = 0x22
  plus = 0xBB; minus = 0xBD; comma = 0xBC; period = 0xBE }
# R-9 (run 13: 'unknown key semicolon'): punctuation by key position (US names; on the German layout
# semicolon = Ue, apostrophe = Ae, openbracket = sz ... - the keys the program's German shortcuts use)
$VK += @{ semicolon = 0xBA; apostrophe = 0xDE; quote = 0xDE; slash = 0xBF; backslash = 0xDC; openbracket = 0xDB; closebracket = 0xDD; backtick = 0xC0; equals = 0xBB }
# R-16 (run 20): PowerShell names are case-insensitive and scoped dynamically - a caller's local `$vk` hid this table
# ("[System.UInt16] does not contain a method named 'ContainsKey'"); always the script's table, never a caller's variable
function VkOf([string]$k) { $k = $k.ToLower(); if ($script:VK.ContainsKey($k)) { return [uint16]$script:VK[$k] }; if ($k.Length -eq 1) { return [uint16][int][char]$k.ToUpper() }; if ($k -match '^f(\d+)$') { return [uint16](0x6F + [int]$matches[1]) }; if ($k -match '^0x') { return [uint16][Convert]::ToInt32($k, 16) }; throw "unknown key $k" }
# R-27 (TW-179): path of an unmodified key - 'scancode' (default: scan code only, down and up as two inputs 40 ms
# apart, like a real keyboard) or 'vk' (virtual key + scan code in one batch, the path up to R-26); init `plain_keys`
# sets the default, a command's `path` overrides it. Keys with Ctrl / Shift / Alt always take the vk path (it works).
function PlainPath([string]$p) { $x = if ($p) { $p } elseif ($S.plain_keys) { [string]$S.plain_keys } else { 'scancode' }; $x.ToLower() }
# R-27 (TW-179): modifiers and main key of a combo. Always arrays - for a single key PowerShell made the pipeline
# result a plain string, so $parts[-1] was the key name's LAST CHARACTER: every unmodified named key (R-13b..R-26)
# went out as a letter key - delet-E = Eraser, righ-T = Text, dow-N = Line, escap-E = Eraser, ente-R, es-C = Crop
# (the R-15 "last letter" finding; single letters and modified combos were right)
function ComboParts([string]$c) {
  $parts = @($c.Split('+') | ? { $_ })
  if (-not $parts.Count) { throw "empty key combo '$c'" }
  $mods = @(); if ($parts.Count -gt 1) { $mods = @($parts[0..($parts.Count - 2)]) }
  @{ mods = $mods; main = [string]$parts[$parts.Count - 1] } }
function Combo([string]$c, [string]$path) {
  $cp = ComboParts $c; $mods = @($cp.mods); $main = $cp.main
  $low = @(@($mods) + @($main) | % { $_.ToLower() })
  if ($low -contains 'ctrl' -and $low -contains 'alt' -and $low -contains 'space') { throw 'ctrl+alt+space is forbidden' }
  $mv = VkOf $main
  if ($mods.Count -eq 0 -and (PlainPath $path) -eq 'scancode') { $dn = [PR.N]::KeySc($mv, $false)
    if ($dn -ne $null) { $up = [PR.N]::KeySc($mv, $true); [PR.N]::Send(@($dn)); Start-Sleep -Milliseconds 40; [PR.N]::Send(@($up))
      return @{ path = 'scancode'; vk = [int]$mv; scan = [int]$dn.u.ki.wScan; ext = [int]($dn.u.ki.dwFlags -band 1) } } }
  $a = @(); foreach ($m in $mods) { $a += [PR.N]::Key((VkOf $m), $false) }
  $md = [PR.N]::Key($mv, $false); $a += $md; $a += [PR.N]::Key($mv, $true)
  for ($i = $mods.Count - 1; $i -ge 0; $i--) { $a += [PR.N]::Key((VkOf $mods[$i]), $true) }
  [PR.N]::Send($a)
  @{ path = 'vk'; vk = [int]$mv; scan = [int]$md.u.ki.wScan; ext = [int]($md.u.ki.dwFlags -band 1); mods = @($mods) }
}
function Mods($mods, [bool]$up) { $a = @(); foreach ($m in @($mods)) { if ($m) { $a += [PR.N]::Key((VkOf $m), $up) } }; if ($a.Count) { [PR.N]::Send($a) } }
function Click($x, $y, $button, $mods, $count) {
  [PR.N]::SetCursorPos([int]$x, [int]$y) | Out-Null; Start-Sleep -Milliseconds 60
  [PR.N]::Send(@([PR.N]::Mouse(0x1, 0)))
  Mods $mods $false
  $d, $u = 0x2, 0x4; if ($button -eq 'right') { $d, $u = 0x8, 0x10 } elseif ($button -eq 'middle') { $d, $u = 0x20, 0x40 }
  if ([int]$count -ge 2) {
    # R-15 (TW-140, tool-text/06: the pointer double-click opened no editor): all clicks of a multi-click in ONE
    # SendInput batch, well inside the system double-click time (GetDoubleClickTime, reported by `key_echo`)
    $a = @(); for ($i = 0; $i -lt [int]$count; $i++) { $a += [PR.N]::Mouse($d, 0); $a += [PR.N]::Mouse($u, 0) }; [PR.N]::Send($a); Start-Sleep -Milliseconds 50
  } else { [PR.N]::Send(@([PR.N]::Mouse($d, 0))); Start-Sleep -Milliseconds 70; [PR.N]::Send(@([PR.N]::Mouse($u, 0))); Start-Sleep -Milliseconds 50 }
  Mods $mods $true; Start-Sleep -Milliseconds 120
}
# R-9 (TW-109): are all given keys down right now (GetAsyncKeyState, also set by injected input)?
function KeysDown($mods) {
  foreach ($m in @($mods)) { if ($m -and ((([int][PR.N]::GetAsyncKeyState([int](VkOf $m))) -band 0x8000) -eq 0)) { return $false } }
  return $true
}
function Drag($x1, $y1, $x2, $y2, $mods, $steps, $button, $pressMs, $releaseMs, $pauseMs) {
  if (-not $steps) { $steps = 20 }
  $d, $u = 0x2, 0x4; if ($button -eq 'right') { $d, $u = 0x8, 0x10 }
  [PR.N]::SetCursorPos([int]$x1, [int]$y1) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 80
  $script:DragKeys = @{ before = (KeysDown $mods); resent = 0 }
  Mods $mods $false
  # R-9 (TW-109, run 13: Alt did not arrive for the Alt drag): check the held keys before the press, send again
  if (@($mods | ? { $_ }).Count) { Start-Sleep -Milliseconds 40
    for ($k = 0; $k -lt 2 -and -not (KeysDown $mods); $k++) { Mods $mods $false; $script:DragKeys.resent++; Start-Sleep -Milliseconds 80 } }
  $script:DragKeys.at_press = (KeysDown $mods)
  # R-5 (run 9, xs-canvas/08,12): on the large bitmap the program is busy after the press; the moves of the drag
  # were coalesced into the final one, and as the program evaluates the second-to-last position the object did not
  # move at all. Now every input waits until the UI thread takes messages again (no effect on a responsive program)
  $busy = 0
  [PR.N]::Send(@([PR.N]::Mouse($d, 0))); Start-Sleep -Milliseconds 60; $busy += [PR.N]::Idle(2000)
  # R-23 (TW-146 rest, run 25 tool-text/13: the drag over the text left no selection): optional pause after the
  # press without any move, so the program takes the press before the first move
  if ($pauseMs) { Start-Sleep -Milliseconds ([int]$pauseMs) }
  # R-7 (M9, panel-library-styles/03: the library drag never arrived): optional pause after the press and two small
  # moves past the drag threshold before the path (drag-and-drop out of a list), optional pause before the release
  if ($pressMs) { Start-Sleep -Milliseconds ([int]$pressMs)
    foreach ($o in 3, 6) { [PR.N]::SetCursorPos([int]$x1 + $o, [int]$y1 + $o) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 80 } }
  for ($i = 1; $i -le $steps; $i++) {
    [PR.N]::SetCursorPos([int]($x1 + ($x2 - $x1) * $i / $steps), [int]($y1 + ($y2 - $y1) * $i / $steps)) | Out-Null
    [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 20; $busy += [PR.N]::Idle(2000)
  }
  # R-4 (RE-6, re 36): the program evaluates only the second-to-last mouse position of a drag (auto shapes came out
  # 1/20 too small): wait, then one more move at the target before releasing
  Start-Sleep -Milliseconds 60
  [PR.N]::SetCursorPos([int]$x2, [int]$y2) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 30; $busy += [PR.N]::Idle(2000)
  if ($releaseMs) { Start-Sleep -Milliseconds ([int]$releaseMs); [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 30 }
  $script:DragKeys.before_release = (KeysDown $mods)
  [PR.N]::Send(@([PR.N]::Mouse($u, 0))); Start-Sleep -Milliseconds 60; $busy += [PR.N]::Idle(3000)
  Mods $mods $true; Start-Sleep -Milliseconds 150
  $busy
}

# ------------------------------------------------------------------ UI Automation
function El([IntPtr]$h) { $UIA::FromHandle($h) }
function ElInfo($e) { $i = $e.Current; $b = $i.BoundingRectangle
  @{ name = $i.Name; type = $i.ControlType.ProgrammaticName -replace '^ControlType\.', ''; class = $i.ClassName; auto_id = $i.AutomationId; enabled = $i.IsEnabled
     rect = $(if ($b.IsEmpty) { $null } else { @([int]$b.Left, [int]$b.Top, [int]$b.Right, [int]$b.Bottom) }) } }
function Roots($scope) {
  $m = MainWin
  if ($scope -eq 'main') { return @(El $m) }
  # 'tops': only the other top-level windows (dialogs) - the main window's tree is large and slow to walk
  if ($scope -eq 'tops') { $l = @(); foreach ($h in [PR.N]::Tops([uint32]$S.pid)) { if ($h -ne $m) { $l += El $h } }; return $l }
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
      if ((MenuWins).Count) { [PR.N]::SetCursorPos($x, $y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 200; Combo 'enter' 'vk' | Out-Null; Start-Sleep -Milliseconds 300 }
      if ((MenuWins).Count) { throw "menu item did not execute (menu still open): $want" }
      $invoked = $true
    } else {
      [PR.N]::SetCursorPos($x, $y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 500
      if ($hit.submenu -and -not $last) { } elseif ($hit.submenu) { Start-Sleep -Milliseconds 200 }
    }
  }
  @{ menus = @(MenuWins | % { WRect $_ }); invoked = $invoked }
}
# R-15 (TW-140): the document view window (largest child of the view class), the target's keyboard layout, a key self test
function ViewHwnd { $m = MainWin; $best = [IntPtr]::Zero; $area = 0
  foreach ($h in [PR.N]::Kids($m)) { if ([PR.N]::Cls($h) -eq $S.view_class) { $r = CRect $h; $a = ($r[2] - $r[0]) * ($r[3] - $r[1]); if ($a -gt $area) { $area = $a; $best = $h } } }
  $best }
function Hex($p) { '0x{0:X8}' -f ([int64]$p -band 0xFFFFFFFF) }
function TargetHkl { [PR.N]::GetKeyboardLayout([PR.N]::Tid((MainWin))) }
function ClsOf($h) { if ($h -eq [IntPtr]::Zero) { return '' }; [PR.N]::Cls($h) }
function HklOf($h) { if ($h -eq [IntPtr]::Zero) { return (TargetHkl) }; [PR.N]::GetKeyboardLayout([PR.N]::Tid($h)) }
function LangDiffers($hkl, [string]$want) { $want -and (([int64]$hkl -band 0xFFFF) -ne ([Convert]::ToInt64($want, 16) -band 0xFFFF)) }
# R-17 (TW-140): WM_INPUTLANGCHANGEREQUEST to every given window (main window and the window with the keyboard focus)
function LangRequest($wins, [string]$want) { $hkl = [PR.N]::LoadKeyboardLayout($want, 0); if ($hkl -eq [IntPtr]::Zero) { return "LoadKeyboardLayout $want failed" }
  foreach ($w in @($wins | ? { $_ -ne [IntPtr]::Zero } | Select -Unique)) { [PR.N]::PostMessage($w, 0x0050, [IntPtr]::Zero, $hkl) | Out-Null }; Start-Sleep -Milliseconds 300; (Hex $hkl) }
function Layout($want) {
  $m = MainWin; $foc = [PR.N]::FocusOf($m); $before = TargetHkl; $fb = HklOf $foc
  $out = @{ before = (Hex $before); requested = $null; focus_class = (ClsOf $foc); focus_before = (Hex $fb) }
  # `want` = KLID ("00000407"): requested for the target's main window and (R-17) the window holding its keyboard
  # focus (WM_INPUTLANGCHANGEREQUEST, like the user switching the input language); nothing is changed when the
  # language of both already matches
  if ((LangDiffers $before $want) -or (LangDiffers $fb $want)) { $out.requested = LangRequest @($m, $foc) $want }
  $after = TargetHkl; $foc = [PR.N]::FocusOf($m); $fa = HklOf $foc; [PR.N]::KeyHkl = $fa
  $out.after = (Hex $after); $out.focus_after = (Hex $fa); $out.host = (Hex ([PR.N]::GetKeyboardLayout(0))); $out }
# R-17 (TW-140): evidence before every plain key - foreground window, keyboard focus (GetGUIThreadInfo of the
# foreground thread when it belongs to the target, else of the main window) and the focus thread's keyboard layout;
# with `want` and another language: WM_INPUTLANGCHANGEREQUEST to the focused window (and the main window), read again
function KeyProbe([string]$want) {
  $fg = [PR.N]::GetForegroundWindow(); $m = MainWin; $own = ($fg -ne [IntPtr]::Zero -and [PR.N]::Pid($fg) -eq [uint32]$S.pid)
  $foc = [PR.N]::FocusOf($(if ($own) { $fg } else { $m })); $hkl = HklOf $foc
  $out = @{ fg_class = (ClsOf $fg); fg_own = $own; focus_class = (ClsOf $foc); focus_hwnd = [int64]$foc; layout = (Hex $hkl); requested = $null }
  if (LangDiffers $hkl $want) { $out.requested = LangRequest @($foc, $m) $want; $hkl = HklOf $foc; $out.layout_after = (Hex $hkl); $out.ok = -not (LangDiffers $hkl $want) }
  [PR.N]::KeyHkl = $hkl; $out }
# R-16: `$mock` (offline self test, selftest_host.ps1): no hook, no SendInput - the inputs that would be sent are
# echoed back as the hook would record them (injected flag set). Locals never reuse script names ($VK, $S: same
# variable for PowerShell, see VkOf).
# R-27 (TW-179): `path` as for `key` (default the init's `plain_keys`): with 'scancode' every key goes out as scan code
# only (wVk = 0) exactly like `key` sends an unmodified key, and the echo proves that the system derived the expected
# virtual key from it (arrived vk = the key's vk); rows carry `path`
function KeyEcho($keys, [switch]$mock, [string]$path) {
  $hkl = TargetHkl; [PR.N]::KeyHkl = $hkl; $m = MainWin; $foc = [PR.N]::FocusOf($m); $pp = PlainPath $path
  if (-not $mock) { $err = [PR.E]::Start(); if ($err) { [PR.E]::Stop(); throw "key_echo: $err" } }
  $sent = @(); $fake = @()
  try { foreach ($k in @($keys)) { $kv = VkOf $k; $sc = $false; $dn = $null
          if ($pp -eq 'scancode') { $dn = [PR.N]::KeySc($kv, $false) }
          if ($dn -ne $null) { $sc = $true; $up = [PR.N]::KeySc($kv, $true) } else { $dn = [PR.N]::Key($kv, $false); $up = [PR.N]::Key($kv, $true) }
          $sent += ,@($k, [int]$kv, [int]$dn.u.ki.wScan, [int]($dn.u.ki.dwFlags -band 1), $(if ($sc) { 'scancode' } else { 'vk' }), [int]$dn.u.ki.wVk)
          if ($mock) { foreach ($in in @($dn, $up)) { $fl = $in.u.ki.dwFlags
              # the system's scan code -> virtual key translation is mocked with the key's own vk
              $fake += ,[uint32[]]@($(if ($fl -band 2) { 0x101 } else { 0x100 }), $(if ($sc) { $kv } else { $in.u.ki.wVk }), $in.u.ki.wScan, (($fl -band 1) -bor 0x10)) } }
          elseif ($sc) { [PR.N]::Send(@($dn)); Start-Sleep -Milliseconds 40; [PR.N]::Send(@($up)); Start-Sleep -Milliseconds 80 }
          else { [PR.N]::Send(@($dn, $up)); Start-Sleep -Milliseconds 80 } }
        if (-not $mock) { Start-Sleep -Milliseconds 200 } } finally { if (-not $mock) { [PR.E]::Stop() } }
  $all = if ($mock) { $fake } else { @([PR.E]::Take()) }; $got = @($all | ? { $_[0] -eq 0x100 -or $_[0] -eq 0x104 }); $rows = @(); $ok = $true
  for ($i = 0; $i -lt $sent.Count; $i++) { $sk = $sent[$i]; $g = if ($i -lt $got.Count) { $got[$i] } else { $null }
    $row = @{ key = $sk[0]; path = $sk[4]; sent = @{ vk = $sk[1]; scan = $sk[2]; ext = $sk[3]; wvk = $sk[5] } }
    if ($g) { $ext = [int]($g[3] -band 1); $row.arrived = @{ vk = [int]$g[1]; scan = [int]$g[2]; ext = $ext; injected = [bool]($g[3] -band 0x10) }
      $row.name = [PR.N]::KeyName($g[2], [bool]$ext, $hkl); $row.char = [PR.N]::KeyChar($g[1], $g[2], $hkl)
      $row.ok = ([int]$g[1] -eq $sk[1] -and [int]$g[2] -eq $sk[2] -and $ext -eq $sk[3]) } else { $row.ok = $false; $row.arrived = $null }
    if (-not $row.ok) { $ok = $false }; $rows += $row }
  @{ ok = $ok; path = $pp; layout = (Hex $hkl); host_layout = (Hex ([PR.N]::GetKeyboardLayout(0))); focus_class = $(if ($foc -ne [IntPtr]::Zero) { [PR.N]::Cls($foc) } else { '' })
     events = $all.Count; keys = $rows; dbl_ms = [int][PR.N]::GetDoubleClickTime(); dbl_cx = [PR.N]::GetSystemMetrics(36); dbl_cy = [PR.N]::GetSystemMetrics(37) } }
function Handle($c) {
  switch ($c.cmd) {
    'init' { if ($c.settle_ms) { $S.settle_ms = [int]$c.settle_ms }; if ($c.plain_keys) { $S.plain_keys = [string]$c.plain_keys }; $S.process = $c.process; $S.main_class = [string]$c.main_class; $S.view_class = [string]$c.view_class; $S.panel_class = [string]$c.panel_class; $S.forbidden = @($c.forbidden_menu | ? { $_ } | % { (($_ -split '>') | % { Clean $_ }) -join ' > ' }); [PR.N]::TargetClass = $S.main_class; $p = Proc; try { [PR.N]::KeyHkl = TargetHkl } catch { }; return @{ pid = $S.pid } }
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
    'focus' { return (Focus ([bool]$c.hard)) }
    'fg' { $f = [PR.N]::GetForegroundWindow(); return @{ hwnd = [int64]$f; class = [PR.N]::Cls($f); pid = [PR.N]::Pid($f); title = [PR.N]::Txt($f); proc = [PR.N]::ProcName($f); guard = [PR.N]::Guard() } }
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
      # negative index = counted from the last match (RUN-6: last slider arrow of an effect popup)
      $hits = New-Object Collections.ArrayList
      foreach ($r in $roots) { foreach ($h in [PR.N]::Kids($r)) {
        if ($c.class -and [PR.N]::Cls($h) -ne $c.class) { continue }
        if ($c.title -and (Clean ([PR.N]::Txt($h))) -ne (Clean $c.title)) { continue }
        [void]$hits.Add($h); if ([int]$c.index -ge 0 -and $hits.Count -gt [int]$c.index) { break } } }
      $k = [int]$c.index; if ($k -lt 0) { $k = $hits.Count + $k }
      if ($k -ge 0 -and $k -lt $hits.Count) { $h = $hits[$k]; $i = WinInfo $h; $i.title = [PR.N]::Txt($h); return $i }
      throw "child not found: $($c | ConvertTo-Json -Compress)" }
    'set_text' { [PR.N]::SendMessageS([IntPtr][int64]$c.hwnd, 0x000C, [IntPtr]::Zero, [string]$c.text) | Out-Null; return 'ok' }
    'next_ctl_root' { return [int64][PR.N]::GetAncestor([IntPtr][int64]$c.hwnd, 2) }
    'next_ctl' { # Tab / Shift+Tab inside a dialog: the program's dialogs do not translate Tab (SendInput Tab types a
      # character into the edit), WM_NEXTDLGCTL moves the focus like the dialog manager does (the edit commits on focus loss)
      $h = [PR.N]::GetAncestor([IntPtr][int64]$c.hwnd, 2)
      if ($h -eq [IntPtr]::Zero -or [PR.N]::Cls($h) -ne '#32770') { throw 'next_ctl: control is not inside a dialog' }
      [PR.N]::SendMessage($h, 0x0028, [IntPtr]$(if ($c.back) { 1 } else { 0 }), [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 120
      $f = [PR.N]::FocusOf($h); return @{ dialog = [int64]$h; focus = [int64]$f; focus_class = $(if ($f -ne [IntPtr]::Zero) { [PR.N]::Cls($f) } else { '' }) } }
    'focus_of' { # R-8 (Q-46): keyboard focus of the thread that owns a window (colour popup hex field)
      $f = [PR.N]::FocusOf([IntPtr][int64]$c.hwnd); return @{ focus = [int64]$f; focus_class = $(if ($f -ne [IntPtr]::Zero) { [PR.N]::Cls($f) } else { '' }) } }
    'get_text' { $sb = New-Object Text.StringBuilder 1024; [PR.N]::SendMessageSB([IntPtr][int64]$c.hwnd, 0x000D, [IntPtr]1024, $sb) | Out-Null; return $sb.ToString() }
    'lb_info' { # owner-drawn list box (layers panel rows): count, top index, selected rows, item heights
      $h = [IntPtr][int64]$c.hwnd; $n = I32 ([PR.N]::SendMessage($h, 0x18B, [IntPtr]::Zero, [IntPtr]::Zero))
      if ($c.top -ne $null) { [PR.N]::SendMessage($h, 0x197, [IntPtr][int]$c.top, [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 150 }
      $sel = @(); $hs = @()
      for ($i = 0; $i -lt $n; $i++) { if ((I32 ([PR.N]::SendMessage($h, 0x187, [IntPtr]$i, [IntPtr]::Zero))) -gt 0) { $sel += $i }
        $hs += (I32 ([PR.N]::SendMessage($h, 0x1A1, [IntPtr]$i, [IntPtr]::Zero))) }
      return @{ count = $n; client = (CRect $h); top = (I32 ([PR.N]::SendMessage($h, 0x18E, [IntPtr]::Zero, [IntPtr]::Zero))); cursel = (I32 ([PR.N]::SendMessage($h, 0x188, [IntPtr]::Zero, [IntPtr]::Zero))); selected = $sel; heights = $hs } }
    'combo_items' { $h = [IntPtr][int64]$c.hwnd; $n = [int][PR.N]::SendMessage($h, 0x146, [IntPtr]::Zero, [IntPtr]::Zero); $l = @()
      for ($i = 0; $i -lt $n; $i++) { $sb = New-Object Text.StringBuilder 512; [PR.N]::SendMessageSB($h, 0x148, [IntPtr]$i, $sb) | Out-Null; $l += $sb.ToString() }
      return @{ items = $l; selected = (I32 ([PR.N]::SendMessage($h, 0x147, [IntPtr]::Zero, [IntPtr]::Zero))) } }
    'combo_select' { $h = [IntPtr][int64]$c.hwnd; $i = I32 ([PR.N]::SendMessageS($h, 0x158, [IntPtr](-1), [string]$c.text))
      if ($i -lt 0) { throw "combo entry not found: $($c.text)" }
      [PR.N]::SendMessage($h, 0x14E, [IntPtr]$i, [IntPtr]::Zero) | Out-Null
      # R-7: notification codes to the parent (config orig.combo_notify, default CBN_SELCHANGE = 1; e.g. 1, 9 = CBN_SELENDOK)
      $codes = if ($c.notify) { @($c.notify) } else { @(1) }
      $id = [PR.N]::GetDlgCtrlID($h); foreach ($n in $codes) { [PR.N]::SendMessage([PR.N]::GetParent($h), 0x111, [IntPtr](([int]$n -shl 16) -bor ($id -band 0xFFFF)), $h) | Out-Null }
      return $i }
    'children' { $m = MainWin; return @([PR.N]::Kids($m) | % { WinInfo $_ } | ? { $_.title -or $c.all }) }
    'dialogs' { return @(Dialogs | % { $i = WinInfo $_; $i.texts = DialogTexts $_; $i }) }
    'dialog_kids' { return @(Dialogs | % { $i = WinInfo $_; $i.kids = @([PR.N]::Kids($_) | % { WinInfo $_ }); $i }) }
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
    'key' { $r = Combo $c.combo ([string]$c.path); Start-Sleep -Milliseconds 80; return $r }
    'layout' { return (Layout ([string]$c.want)) }
    'key_probe' { return (KeyProbe ([string]$c.want)) }
    'key_echo' { $k = if ($c.keys) { @($c.keys) } else { @('escape', 'delete', 'right', 'left', 'home', 'end', 'pageup', 'insert', 'c', 'e', 't', 'z') }
      # R-16: the failure goes back to the runner (logged there, not fatal) with the host script line that raised it
      try { return (KeyEcho $k -path ([string]$c.path)) } catch { $ii = $_.InvocationInfo; $src = if ($ii -and $ii.Line) { $ii.Line.Trim() } else { '' }; if ($src.Length -gt 240) { $src = $src.Substring(0, 240) + '...' }
        throw "$($_.Exception.GetType().FullName): $($_.Exception.Message) [orig_host.ps1 line $(if ($ii) { $ii.ScriptLineNumber } else { '?' }), offset $(if ($ii) { $ii.OffsetInLine } else { '?' }): $src]" } }
    'view_focus' { # R-15 (TW-140): keyboard focus to the document view (SetFocus with attached input), no click
      $v = ViewHwnd; if ($v -eq [IntPtr]::Zero) { throw 'view window not found' }
      $m = MainWin; $was = [PR.N]::FocusOf($m); $me = [PR.N]::GetCurrentThreadId(); $tt = [PR.N]::Tid($v)
      if ($tt -and $tt -ne $me) { [PR.N]::AttachThreadInput($me, $tt, $true) | Out-Null }
      try { [PR.N]::SetFocus($v) | Out-Null } finally { if ($tt -and $tt -ne $me) { [PR.N]::AttachThreadInput($me, $tt, $false) | Out-Null } }
      Start-Sleep -Milliseconds 60; $f = [PR.N]::FocusOf($m)
      return @{ view = [int64]$v; was_class = $(if ($was -ne [IntPtr]::Zero) { [PR.N]::Cls($was) } else { '' }); focus_class = $(if ($f -ne [IntPtr]::Zero) { [PR.N]::Cls($f) } else { '' }); ok = ($f -eq $v -or [PR.N]::IsChild($v, $f)) } }
    # R-20 (TW-146): `mode: keys` types every character as its real key (vk + scan code + shift state of the target's
    # layout, Unicode only for characters without a key; LF is sent as Enter), `delay_ms` between characters (default 25)
    # R-27 (TW-179): with mode keys and the scan-code path (`path`, default the init's `plain_keys`) every key of a
    # character (its Shift / Ctrl / Alt too) goes out as scan code only, one input at a time `gap_ms` (default 15) apart
    'type' { $d = if ($c.delay_ms -ne $null) { [int]$c.delay_ms } else { 25 }; $keys = ([string]$c.mode -eq 'keys'); $nk = 0; $nu = 0
      $sc = $keys -and ((PlainPath ([string]$c.path)) -eq 'scancode'); $gap = if ($c.gap_ms -ne $null) { [int]$c.gap_ms } else { 15 }
      foreach ($ch in ([string]$c.text).ToCharArray()) { $a = $null
        if ($keys) { if ($ch -eq [char]10) { $ch = [char]13 }; $a = [PR.N]::CharKeys($ch, $sc) }
        if ($a -ne $null) { if ($sc) { foreach ($in in $a) { [PR.N]::Send(@($in)); Start-Sleep -Milliseconds $gap } } else { [PR.N]::Send($a) }; $nk++ }
        else { [PR.N]::Send(@([PR.N]::Uni($ch, $false), [PR.N]::Uni($ch, $true))); $nu++ }
        Start-Sleep -Milliseconds $d }
      if ($keys) { return @{ keys = $nk; unicode = $nu; delay_ms = $d; path = $(if ($sc) { 'scancode' } else { 'vk' }) } }; return 'ok' }
    'caret' { # R-20 (TW-146): caret window and keyboard focus of the main window's thread (text editor open?)
      $m = MainWin; $g = [PR.N]::GuiOf($m)
      return @{ caret = [int64]$g.hwndCaret; caret_class = (ClsOf $g.hwndCaret); blinking = (($g.flags -band 1) -ne 0); caret_rect = @([int]$g.rcCaret.L, [int]$g.rcCaret.T, [int]$g.rcCaret.R, [int]$g.rcCaret.B); focus_class = (ClsOf $g.hwndFocus) } }
    'click' { Click $c.x $c.y $c.button $c.mods $c.count; return 'ok' }
    'move' { [PR.N]::SetCursorPos([int]$c.x, [int]$c.y) | Out-Null; [PR.N]::Send(@([PR.N]::Mouse(0x1, 0))); Start-Sleep -Milliseconds 120; return 'ok' }
    'drag' { $script:DragKeys = $null; $b = Drag $c.x1 $c.y1 $c.x2 $c.y2 $c.mods $c.steps $c.button $c.press_ms $c.release_ms $c.pause_ms
             $r = @{ ok = $true; busy_ms = [int]$b }; if ($script:DragKeys -and @($c.mods | ? { $_ }).Count) { $r.keys = $script:DragKeys }; return $r }
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
    'shot' { $r = if ($c.rect) { @($c.rect) } else { WRect (MainWin) }; $b = [PR.N]::Screen($r[0], $r[1], $r[2], $r[3]); $b.Save($c.path, [System.Drawing.Imaging.ImageFormat]::Png); $b.Dispose(); return $c.path }
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
