using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using FileTime = System.Runtime.InteropServices.ComTypes.FILETIME;
using System.Web.Script.Serialization;
using System.Windows.Forms;

// Passive receiver only: no shortcuts, hooks, focus transfer or input injection.
internal sealed class HotkeyInputWindow : Form
{
    private const int SharingViolation = 32;
    private const int LockViolation = 33;
    private const int UnableToRemoveReplaced = 1175;
    private const int MaxPublishRetries = 20;
    private const uint QueryLimitedInformation = 0x1000;
    private const int IdentityBufferSize = 32768;
    private const int InvalidIdentity = 13;
    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);

    [DllImport("user32.dll")]
    private static extern IntPtr GetMessageExtraInfo();

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(uint access, bool inherit, uint processId);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool QueryFullProcessImageName(IntPtr process, uint flags, StringBuilder name, ref uint size);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetProcessTimes(IntPtr process, out FileTime creation, out FileTime exit,
        out FileTime kernel, out FileTime user);

    [DllImport("kernel32.dll")]
    private static extern bool CloseHandle(IntPtr handle);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr window, StringBuilder title, int size);

    private readonly string root;
    private readonly string role;
    private readonly string token;
    private readonly int pid;
    private readonly uint observedGamePid;
    private sealed class ForegroundIdentity
    {
        public uint pid;
        public string executable;
        public string title;
        public string creationTime;
        public int error;
    }
    private sealed class KeyReceipt
    {
        public string key;
        public bool control;
        public bool alt;
        public bool shift;
        public uint foregroundPid;
        public string at;
    }
    private sealed class FocusObservation
    {
        public uint pid;
        public string at;
    }
    private sealed class KeyTransition
    {
        public string key;
        public string direction;
        public string messageExtraInfo;
        public bool control;
        public bool alt;
        public bool shift;
        public uint foregroundPid;
        public string at;
    }
    private readonly List<KeyReceipt> keys = new List<KeyReceipt>();
    private readonly List<FocusObservation> focus = new List<FocusObservation>();
    private readonly List<KeyTransition> transitions = new List<KeyTransition>();
    private readonly JavaScriptSerializer json = new JavaScriptSerializer();
    private readonly System.Windows.Forms.Timer timer = new System.Windows.Forms.Timer();
    private readonly Label status = new Label();
    private int publishRetries;
    private int mouseClicks;

    private HotkeyInputWindow(string fixtureRoot, string cleanupToken, string targetRole, uint gamePid)
    {
        root = fixtureRoot;
        token = cleanupToken;
        role = targetRole;
        observedGamePid = gamePid;
        using (var process = Process.GetCurrentProcess()) pid = process.Id;
        Text = "EMMM Hotkey " + role + " " + token;
        ClientSize = role == "game" ? new Size(800, 560) : new Size(570, 170);
        KeyPreview = true;
        status.Dock = DockStyle.Fill;
        status.Padding = new Padding(12);
        Controls.Add(status);
        status.MouseDown += delegate { mouseClicks++; Publish(); };
        KeyDown += delegate(object sender, KeyEventArgs input)
        {
            RecordTransition(input, "down");
            if (input.KeyCode >= Keys.F13 && input.KeyCode <= Keys.F18)
            {
                if (keys.Count == 128) throw new InvalidOperationException("Input evidence limit reached");
                uint foregroundPid;
                GetWindowThreadProcessId(GetForegroundWindow(), out foregroundPid);
                keys.Add(new KeyReceipt { key = input.KeyCode.ToString(), control = input.Control,
                    alt = input.Alt, shift = input.Shift, foregroundPid = foregroundPid,
                    at = DateTime.UtcNow.ToString("o") });
                Publish();
            }
        };
        KeyUp += delegate(object sender, KeyEventArgs input) { RecordTransition(input, "up"); };
        timer.Interval = 100;
        timer.Tick += delegate
        {
            if (File.Exists(Path.Combine(root, role + ".stop"))) Close();
            else Publish();
        };
        Shown += delegate { Publish(); timer.Start(); };
        FormClosing += delegate(object sender, FormClosingEventArgs closing)
        {
            File.WriteAllText(Path.Combine(root, role + ".closed.json"), json.Serialize(new {
                pid = pid, at = DateTime.UtcNow.ToString("o"), reason = closing.CloseReason.ToString(),
                stopRequested = File.Exists(Path.Combine(root, role + ".stop"))
            }));
        };
        FormClosed += delegate { timer.Stop(); timer.Dispose(); };
    }

    private void RecordTransition(KeyEventArgs input, string direction)
    {
        if (!(input.KeyCode >= Keys.F13 && input.KeyCode <= Keys.F18)
            && input.KeyCode != Keys.ControlKey && input.KeyCode != Keys.ShiftKey
            && input.KeyCode != Keys.Menu && input.KeyCode != Keys.LControlKey
            && input.KeyCode != Keys.RControlKey && input.KeyCode != Keys.LShiftKey
            && input.KeyCode != Keys.RShiftKey && input.KeyCode != Keys.LMenu
            && input.KeyCode != Keys.RMenu) return;
        if (transitions.Count == 512) throw new InvalidOperationException("Key transition evidence limit reached");
        uint foregroundPid;
        GetWindowThreadProcessId(GetForegroundWindow(), out foregroundPid);
        transitions.Add(new KeyTransition { key = input.KeyCode.ToString(), direction = direction,
            messageExtraInfo = GetMessageExtraInfo().ToInt64().ToString("x16"),
            control = input.Control, alt = input.Alt, shift = input.Shift,
            foregroundPid = foregroundPid, at = DateTime.UtcNow.ToString("o") });
    }

    private void Publish()
    {
        uint foregroundPid;
        IntPtr foregroundWindow = GetForegroundWindow();
        GetWindowThreadProcessId(foregroundWindow, out foregroundPid);
        if (focus.Count == 0 || focus[focus.Count - 1].pid != foregroundPid)
        {
            if (focus.Count == 256) throw new InvalidOperationException("Focus evidence limit reached");
            focus.Add(new FocusObservation { pid = foregroundPid, at = DateTime.UtcNow.ToString("o") });
        }
        status.Text = "Owned passive test receiver\r\nRole: " + role +
            "\r\nPID: " + pid + " / foreground: " + foregroundPid +
            "\r\nReceived function key count: " + keys.Count;
        string destination = Path.Combine(root, role + ".status.json");
        string temporary = destination + ".tmp";
        File.WriteAllText(temporary, json.Serialize(new {
            token = token, role = role, pid = pid, foregroundPid = foregroundPid,
            keys = keys.ToArray(), transitions = transitions.ToArray(),
            mouseClicks = mouseClicks,
            foregroundIdentity = ObserveForeground(foregroundWindow, foregroundPid),
            focus = focus.ToArray(), at = DateTime.UtcNow.ToString("o")
        }));
        try
        {
            if (File.Exists(destination)) File.Replace(temporary, destination, null);
            else File.Move(temporary, destination);
            publishRetries = 0;
        }
        catch (IOException error)
        {
            int code = error.HResult & 0xffff;
            if (code != SharingViolation && code != LockViolation && code != UnableToRemoveReplaced) throw;
            // Readers can briefly deny replacement. Keep receipts in memory and
            // publish the latest complete snapshot on the next UI timer tick.
            if (++publishRetries > MaxPublishRetries)
                throw new IOException("Owned status publication stayed locked past its retry bound", error);
        }
    }

    private ForegroundIdentity ObserveForeground(IntPtr window, uint foregroundPid)
    {
        if (observedGamePid == 0 || foregroundPid != observedGamePid) return null;
        var identity = new ForegroundIdentity { pid = foregroundPid, executable = null,
            title = null, creationTime = null, error = 0 };
        IntPtr process = OpenProcess(QueryLimitedInformation, false, foregroundPid);
        if (process == IntPtr.Zero) { identity.error = Marshal.GetLastWin32Error(); return identity; }
        try
        {
            uint size = IdentityBufferSize;
            var executable = new StringBuilder(IdentityBufferSize);
            FileTime creation, exit, kernel, user;
            if (!QueryFullProcessImageName(process, 0, executable, ref size)
                || !GetProcessTimes(process, out creation, out exit, out kernel, out user))
            { identity.error = Marshal.GetLastWin32Error(); return identity; }
            if (!String.Equals(Path.GetFileName(executable.ToString()), "GenshinImpact.exe",
                StringComparison.OrdinalIgnoreCase))
            { identity.error = InvalidIdentity; return identity; }
            var title = new StringBuilder(IdentityBufferSize);
            if (GetWindowText(window, title, title.Capacity) == 0)
            { identity.error = InvalidIdentity; return identity; }
            uint currentPid;
            GetWindowThreadProcessId(GetForegroundWindow(), out currentPid);
            if (currentPid != foregroundPid)
            { identity.error = InvalidIdentity; return identity; }
            identity.executable = executable.ToString();
            identity.title = title.ToString();
            identity.creationTime = (((ulong)(uint)creation.dwHighDateTime << 32)
                | (uint)creation.dwLowDateTime).ToString("x16");
            return identity;
        }
        finally { CloseHandle(process); }
    }

    [STAThread]
    private static int Main(string[] args)
    {
        try
        {
            uint gamePid = 0;
            if ((args.Length != 3 && args.Length != 4) || (args[2] != "game" && args[2] != "other")
                || (args.Length == 4 && (!uint.TryParse(args[3], out gamePid) || gamePid == 0)))
                throw new ArgumentException("Expected owned root, token and game/other role");
            string fixtureRoot = Path.GetFullPath(args[0]).TrimEnd(Path.DirectorySeparatorChar);
            string temp = Path.GetFullPath(Path.GetTempPath()).TrimEnd(Path.DirectorySeparatorChar);
            if (!fixtureRoot.StartsWith(temp + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase)
                || !Path.GetFileName(fixtureRoot).StartsWith("EMMM_", StringComparison.Ordinal)
                || (File.GetAttributes(fixtureRoot) & FileAttributes.ReparsePoint) != 0
                || !String.Equals(AppDomain.CurrentDomain.BaseDirectory.TrimEnd(Path.DirectorySeparatorChar),
                    fixtureRoot, StringComparison.OrdinalIgnoreCase)
                || File.ReadAllText(Path.Combine(fixtureRoot, ".emmm-e2e-fixture")) != args[1])
                throw new InvalidOperationException("Owned fixture identity validation failed");
            if (File.Exists(Path.Combine(fixtureRoot, args[2] + ".status.json")))
                throw new InvalidOperationException("Refusing a reused receiver evidence path");
            Application.SetUnhandledExceptionMode(UnhandledExceptionMode.ThrowException);
            Application.EnableVisualStyles();
            using (var form = new HotkeyInputWindow(fixtureRoot, args[1], args[2], gamePid)) Application.Run(form);
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error);
            return 1;
        }
    }
}
