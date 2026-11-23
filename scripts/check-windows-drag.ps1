param(
    [Parameter(Mandatory)][long]$Window,
    [Parameter(Mandatory)][int]$ClientX,
    [Parameter(Mandatory)][int]$ClientY,
    [Parameter(Mandatory)][string]$Path
)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') {
    throw 'Pointer automation requires an isolated Windows runner'
}

Add-Type -ReferencedAssemblies System.Windows.Forms.dll,System.Drawing.dll -TypeDefinition @'
using System;
using System.Collections.Specialized;
using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;

public static class InstallerFileDrag
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Point { public int X; public int Y; }

    [StructLayout(LayoutKind.Sequential)]
    private struct MouseInput
    {
        public int X;
        public int Y;
        public uint Wheel;
        public uint Flags;
        public uint Time;
        public UIntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Input { public uint Type; public MouseInput Mouse; }

    [DllImport("user32.dll")] private static extern bool ClientToScreen(IntPtr window, ref Point point);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] private static extern short GetAsyncKeyState(int key);
    [DllImport("user32.dll")] private static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32.dll")] private static extern IntPtr GetAncestor(IntPtr window, uint flags);
    [DllImport("user32.dll")] private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint count, Input[] input, int size);

    private static void Press(uint flags)
    {
        var mouse = new Input { Mouse = new MouseInput { Flags = flags } };
        if (SendInput(1, new[] { mouse }, Marshal.SizeOf(typeof(Input))) != 1)
            throw new InvalidOperationException("Windows refused the drag pointer input");
    }

    public static void Drop(IntPtr window, int x, int y, string path)
    {
        SetThreadDpiAwarenessContext(new IntPtr(-4)); // Per-monitor aware v2
        SetForegroundWindow(window);
        var point = new Point { X = x, Y = y };
        if (!ClientToScreen(window, ref point) || GetAncestor(WindowFromPoint(point), 2) != window)
            throw new InvalidOperationException("The drag target is outside the installed Khor window");
        using (var source = new Form())
        {
            var screen = Screen.FromPoint(new System.Drawing.Point(point.X, point.Y)).WorkingArea;
            source.Text = "Khor drag source";
            source.ShowInTaskbar = false;
            source.TopMost = true;
            source.StartPosition = FormStartPosition.Manual;
            source.Bounds = new Rectangle(screen.Left + 12, screen.Top + 12, 190, 95);
            var paths = new StringCollection();
            paths.Add(path);
            var data = new DataObject();
            data.SetFileDropList(paths);
            bool finished = false;
            Exception failure = null;
            source.MouseDown += (sender, mouse) =>
            {
                if (mouse.Button != MouseButtons.Left) return;
                Exception gestureError = null;
                var gesture = new Thread(() =>
                {
                    try
                    {
                        Thread.Sleep(350);
                        if (!SetCursorPos(point.X - 6, point.Y))
                            throw new InvalidOperationException("Windows refused to move the drag pointer");
                        Thread.Sleep(150);
                        if (!SetCursorPos(point.X, point.Y))
                            throw new InvalidOperationException("Windows refused to reach the drop target");
                        Thread.Sleep(350);
                        Press(0x0004); // Left up commits the OLE drop.
                    }
                    catch (Exception error) { gestureError = error; }
                });
                gesture.IsBackground = true;
                gesture.Start();
                try
                {
                    var watch = Stopwatch.StartNew();
                    var effect = source.DoDragDrop(data, DragDropEffects.Copy);
                    Console.WriteLine("OLE file drag returned {0} after {1} ms", effect, watch.ElapsedMilliseconds);
                    if (watch.ElapsedMilliseconds < 350)
                        throw new InvalidOperationException("The OLE drag ended before the pointer reached Khor");
                }
                catch (Exception error) { failure = error; }
                finally
                {
                    gesture.Join();
                    if (gestureError != null) failure = gestureError;
                    finished = true;
                }
            };
            source.Show();
            source.Activate();
            Application.DoEvents();
            var start = source.PointToScreen(new System.Drawing.Point(source.ClientSize.Width / 2, source.ClientSize.Height / 2));
            if (source.Bounds.Contains(point.X, point.Y) || !SetCursorPos(start.X, start.Y))
                throw new InvalidOperationException("Could not position the native file source");
            try
            {
                Press(0x0002);
                var watch = Stopwatch.StartNew();
                while (!finished && watch.ElapsedMilliseconds < 4000)
                {
                    if ((GetAsyncKeyState(1) & 0x8000) != 0) Application.DoEvents();
                    Thread.Sleep(10);
                }
                if (failure != null) throw failure;
                if (!finished) throw new InvalidOperationException("The drag source did not receive the pointer press");
            }
            finally { Press(0x0004); }
        }
    }
}
'@

[InstallerFileDrag]::Drop([IntPtr]::new($Window), $ClientX, $ClientY, (Resolve-Path -LiteralPath $Path).Path)
