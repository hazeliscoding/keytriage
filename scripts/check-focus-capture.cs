// Win32 helpers for check-focus-capture.ps1, loaded with Add-Type.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Windows.Forms;

namespace FocusCheck
{
    public static class Win
    {
        const uint INPUT_MOUSE = 0, INPUT_KEYBOARD = 1;
        const uint KEYEVENTF_KEYUP = 0x0002, KEYEVENTF_SCANCODE = 0x0008;
        const uint MOUSEEVENTF_LEFTDOWN = 0x0002, MOUSEEVENTF_LEFTUP = 0x0004;
        const int SW_RESTORE = 9;
        const uint WM_NULL = 0x0000, SMTO_ABORTIFHUNG = 0x0002;
        static readonly IntPtr DPI_PER_MONITOR_AWARE_V2 = new IntPtr(-4);

        [StructLayout(LayoutKind.Sequential)]
        struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Sequential)]
        struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Explicit)]
        struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
        [StructLayout(LayoutKind.Sequential)]
        struct INPUT { public uint type; public INPUTUNION u; }
        [StructLayout(LayoutKind.Sequential)]
        struct RECT { public int Left, Top, Right, Bottom; }
        [StructLayout(LayoutKind.Sequential)]
        struct POINT { public int X, Y; }
        [StructLayout(LayoutKind.Sequential)]
        struct GUITHREADINFO { public int cbSize; public uint flags; public IntPtr hwndActive, hwndFocus, hwndCapture, hwndMenuOwner, hwndMoveSize, hwndCaret; public RECT rcCaret; }

        delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lParam);

        [DllImport("user32.dll", SetLastError = true)] static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
        [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hWnd);
        [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
        [DllImport("user32.dll")] static extern bool IsIconic(IntPtr hWnd);
        [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
        [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hWnd);
        [DllImport("user32.dll")] static extern bool IsChild(IntPtr hWndParent, IntPtr hWnd);
        [DllImport("user32.dll", SetLastError = true)] static extern bool GetGUIThreadInfo(uint idThread, ref GUITHREADINFO pgui);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr hWnd, StringBuilder sb, int max);
        [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hWnd, out RECT r);
        [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr hWnd, ref POINT p);
        [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(POINT p);
        [DllImport("user32.dll")] static extern bool GetCursorPos(out POINT p);
        [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
        [DllImport("user32.dll", SetLastError = true)] static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam, uint flags, uint timeoutMs, out IntPtr result);
        [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr ctx);
        [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);

        static int InputSize { get { return Marshal.SizeOf(typeof(INPUT)); } }

        public static string Class(IntPtr h) { var sb = new StringBuilder(256); GetClassName(h, sb, sb.Capacity); return sb.ToString(); }
        static string Text(IntPtr h) { var sb = new StringBuilder(256); GetWindowText(h, sb, sb.Capacity); return sb.ToString(); }
        static uint Pid(IntPtr h) { uint pid; GetWindowThreadProcessId(h, out pid); return pid; }

        // Match the process as well as the title: an Explorer window on the repo folder is also
        // titled "keytriage".
        public static IntPtr FindTopLevel(int pid, string title)
        {
            IntPtr found = IntPtr.Zero;
            EnumWindows((h, l) =>
            {
                if (Pid(h) == (uint)pid && IsWindowVisible(h) && Text(h) == title) { found = h; return false; }
                return true;
            }, IntPtr.Zero);
            return found;
        }

        // Activation across threads is asynchronous, so let the target process it before reading
        // the foreground.
        static bool WaitForeground(IntPtr hwnd, int timeoutMs)
        {
            IntPtr r;
            SendMessageTimeout(hwnd, WM_NULL, IntPtr.Zero, IntPtr.Zero, SMTO_ABORTIFHUNG, (uint)timeoutMs, out r);
            var sw = Stopwatch.StartNew();
            while (sw.ElapsedMilliseconds < timeoutMs)
            {
                if (GetForegroundWindow() == hwnd) return true;
                Thread.Sleep(20);
            }
            return false;
        }

        // Returns how the window was activated, or null. The Alt-key trick is left out: the app
        // would capture the Alt, and a lone Alt opens menu mode.
        public static string Activate(IntPtr hwnd, int timeoutMs)
        {
            if (IsIconic(hwnd)) ShowWindow(hwnd, SW_RESTORE);
            if (GetForegroundWindow() == hwnd) return "already";

            SetForegroundWindow(hwnd);
            if (WaitForeground(hwnd, timeoutMs)) return "plain";

            // PowerToys' workaround: an empty injected mouse event lets the next call through.
            var nudge = new INPUT[1];
            nudge[0].type = INPUT_MOUSE;
            SendInput(1, nudge, InputSize);
            SetForegroundWindow(hwnd);
            if (WaitForeground(hwnd, timeoutMs)) return "zero-input";

            // A click on a background window is a foreground change the system makes itself.
            if (ClickClientCenter(hwnd) && WaitForeground(hwnd, timeoutMs)) return "click";
            return null;
        }

        public static bool ClickClientCenter(IntPtr hwnd)
        {
            SetThreadDpiAwarenessContext(DPI_PER_MONITOR_AWARE_V2);
            RECT rc;
            if (!GetClientRect(hwnd, out rc)) return false;
            var p = new POINT { X = (rc.Right - rc.Left) / 2, Y = (rc.Bottom - rc.Top) / 2 };
            if (!ClientToScreen(hwnd, ref p)) return false;
            IntPtr hit = WindowFromPoint(p);
            if (hit != hwnd && !IsChild(hwnd, hit)) return false;
            POINT saved;
            GetCursorPos(out saved);
            SetCursorPos(p.X, p.Y);
            var click = new INPUT[2];
            click[0].type = INPUT_MOUSE; click[0].u.mi.dwFlags = MOUSEEVENTF_LEFTDOWN;
            click[1].type = INPUT_MOUSE; click[1].u.mi.dwFlags = MOUSEEVENTF_LEFTUP;
            uint sent = SendInput(2, click, InputSize);
            Thread.Sleep(50);
            SetCursorPos(saved.X, saved.Y);
            return sent == 2;
        }

        // The focus window of the foreground thread: where it sits relative to `top`, its class
        // and its process.
        public static string FocusInfo(IntPtr top)
        {
            var g = new GUITHREADINFO();
            g.cbSize = Marshal.SizeOf(typeof(GUITHREADINFO));
            if (!GetGUIThreadInfo(0, ref g) || g.hwndFocus == IntPtr.Zero) return "none";
            IntPtr f = g.hwndFocus;
            string where = f == top ? "top" : (IsChild(top, f) ? "child" : "outside");
            string proc;
            try { proc = Process.GetProcessById((int)Pid(f)).ProcessName; } catch { proc = "?"; }
            return where + " class=" + Class(f) + " process=" + proc;
        }

        // The process that owns the focus window when focus sits inside `top`, or 0.
        public static uint FocusProcess(IntPtr top)
        {
            var g = new GUITHREADINFO();
            g.cbSize = Marshal.SizeOf(typeof(GUITHREADINFO));
            if (!GetGUIThreadInfo(0, ref g) || g.hwndFocus == IntPtr.Zero || !IsChild(top, g.hwndFocus)) return 0;
            return Pid(g.hwndFocus);
        }

        // Taps a key by scan code. The layout picks the virtual key, and Raw Input reports the
        // scan code as MakeCode.
        public static uint Tap(ushort scan, int count)
        {
            var list = new List<INPUT>();
            for (int i = 0; i < count; i++)
            {
                list.Add(Key(scan, false));
                list.Add(Key(scan, true));
            }
            return SendInput((uint)list.Count, list.ToArray(), InputSize);
        }

        static INPUT Key(ushort scan, bool up)
        {
            var k = new INPUT();
            k.type = INPUT_KEYBOARD;
            k.u.ki.wScan = scan;
            k.u.ki.dwFlags = KEYEVENTF_SCANCODE | (up ? KEYEVENTF_KEYUP : 0);
            return k;
        }
    }

    // The other process's window for the background phase. It counts the key downs it receives,
    // which proves the injected keys went somewhere while the app was in the background.
    public sealed class ProbeForm : Form
    {
        const int WM_KEYDOWN = 0x0100;
        public IntPtr Hwnd;
        readonly ushort scan;
        int count;
        public int Count { get { return Volatile.Read(ref count); } }

        // Small, in a corner and topmost, so it never covers the middle of the app window, where a
        // fallback click lands.
        ProbeForm(string title, ushort scan)
        {
            Text = title;
            this.scan = scan;
            Width = 320;
            Height = 160;
            StartPosition = FormStartPosition.Manual;
            Location = new System.Drawing.Point(0, 0);
            TopMost = true;
        }

        public bool WaitCount(int n, int timeoutMs)
        {
            var sw = Stopwatch.StartNew();
            while (sw.ElapsedMilliseconds < timeoutMs) { if (Count >= n) return true; Thread.Sleep(20); }
            return Count >= n;
        }

        protected override void WndProc(ref Message m)
        {
            if (m.Msg == WM_KEYDOWN && (((long)m.LParam >> 16) & 0xFF) == scan) Interlocked.Increment(ref count);
            base.WndProc(ref m);
        }

        public static ProbeForm Start(string title, ushort scan)
        {
            ProbeForm form = null;
            var ready = new ManualResetEvent(false);
            var t = new Thread(() =>
            {
                form = new ProbeForm(title, scan);
                form.Shown += (s, e) => { form.Hwnd = form.Handle; ready.Set(); };
                Application.Run(form);
            });
            t.SetApartmentState(ApartmentState.STA);
            t.IsBackground = true;
            t.Start();
            if (!ready.WaitOne(10000)) throw new TimeoutException("the probe window did not show");
            return form;
        }

        public void CloseFromAnyThread() { BeginInvoke(new Action(Close)); }
    }

    public sealed class Registration
    {
        public long Page, Usage, Flags, Target;
    }

    // The app under test. Its output stays in memory, and callers print counts only.
    public sealed class AppUnderTest : IDisposable
    {
        const int Other = -1;
        static readonly Regex EventLine = new Regex(@"^kt-input: key=(0x[0-9a-f]+|other) up=([01]) device=0x([0-9a-f]+)$");
        static readonly Regex RegisteredLine = new Regex(@"^kt-input: registered page=0x([0-9a-f]+) usage=0x([0-9a-f]+) flags=0x([0-9a-f]+) target=0x([0-9a-f]+)$");

        public Process Proc;
        readonly object gate = new object();
        readonly Dictionary<int, int> downs = new Dictionary<int, int>();
        readonly HashSet<long> devices = new HashSet<long>();
        readonly List<Registration> registrations = new List<Registration>();
        bool ready;

        public static AppUnderTest Start(string exe, bool visualHosting)
        {
            var psi = new ProcessStartInfo(exe);
            psi.UseShellExecute = false;
            psi.RedirectStandardOutput = true;
            // The debug build is a console app and would otherwise open a console that can take
            // the foreground.
            psi.CreateNoWindow = true;
            psi.Environment["KEYTRIAGE_ECHO"] = "1";
            // The hosting mode decides which process holds keyboard focus, so the check sets it
            // instead of inheriting it.
            const string hosting = "COREWEBVIEW2_FORCED_HOSTING_MODE";
            if (visualHosting) psi.Environment[hosting] = "COREWEBVIEW2_HOSTING_MODE_WINDOW_TO_VISUAL";
            else psi.Environment.Remove(hosting);
            var app = new AppUnderTest();
            app.Proc = new Process();
            app.Proc.StartInfo = psi;
            app.Proc.OutputDataReceived += (s, e) => { if (e.Data != null) app.OnLine(e.Data); };
            app.Proc.Start();
            app.Proc.BeginOutputReadLine();
            return app;
        }

        static long Hex(string s) { return Convert.ToInt64(s, 16); }

        void OnLine(string line)
        {
            lock (gate)
            {
                if (line == "kt-input: ready") { ready = true; return; }
                var r = RegisteredLine.Match(line);
                if (r.Success)
                {
                    registrations.Add(new Registration
                    {
                        Page = Hex(r.Groups[1].Value), Usage = Hex(r.Groups[2].Value),
                        Flags = Hex(r.Groups[3].Value), Target = Hex(r.Groups[4].Value),
                    });
                    return;
                }
                var m = EventLine.Match(line);
                if (!m.Success || m.Groups[2].Value != "0") return;
                int key = m.Groups[1].Value == "other" ? Other : (int)Hex(m.Groups[1].Value);
                int n;
                downs.TryGetValue(key, out n);
                downs[key] = n + 1;
                if (key != Other) devices.Add(Hex(m.Groups[3].Value));
            }
        }

        public Registration[] Registrations { get { lock (gate) return registrations.ToArray(); } }
        public int Downs(int key) { lock (gate) { int n; return downs.TryGetValue(key, out n) ? n : 0; } }
        public int OtherDowns { get { return Downs(Other); } }

        // The device handles the injected marker keys arrived with.
        public string MarkerDevices
        {
            get
            {
                lock (gate)
                {
                    var list = new List<string>();
                    foreach (var d in devices) list.Add("0x" + d.ToString("x"));
                    return string.Join(",", list);
                }
            }
        }

        public bool WaitReady(int timeoutMs)
        {
            var sw = Stopwatch.StartNew();
            while (sw.ElapsedMilliseconds < timeoutMs) { lock (gate) if (ready) return true; Thread.Sleep(50); }
            return false;
        }

        public bool WaitDowns(int key, int n, int timeoutMs)
        {
            var sw = Stopwatch.StartNew();
            while (sw.ElapsedMilliseconds < timeoutMs) { if (Downs(key) >= n) return true; Thread.Sleep(20); }
            return Downs(key) >= n;
        }

        public void Dispose()
        {
            try
            {
                if (!Proc.HasExited) { Proc.CloseMainWindow(); if (!Proc.WaitForExit(5000)) Proc.Kill(); }
            }
            catch (InvalidOperationException) { }
        }
    }
}
