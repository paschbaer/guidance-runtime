# Guidance Runtime - provision the GitHub update credential (per user).
#
# Launched by the MSI exit-dialog checkbox. Reads the personal access token
# with masked input and stores it via CredWriteW as the generic credential
# 'guidance:update:<entry>' in the CURRENT user's Credential Manager. The
# token never appears on a command line, in a log, or in the MSI property
# table.

param(
    [string]$Entry = "guidance-update"
)

$ErrorActionPreference = 'Stop'
$target = "guidance:update:$Entry"

Write-Host ""
Write-Host "Guidance Runtime - Update-Zugang fuer private Repositories"
Write-Host "==========================================================="
Write-Host ""
Write-Host "Anzulegende Anmeldeinformation:"
Write-Host "  Ziel : $target"
Write-Host "  Konto: $env:USERNAME  (nur dein Benutzerkonto)"
Write-Host ""
Write-Host "Eingefuegt wird dein GitHub Personal Access Token (github_pat_...),"
Write-Host "NICHT dein GitHub-Passwort - GitHub akzeptiert seit 2020 nur noch"
Write-Host "Tokens fuer programmatische Zugriffe. Die Erstellung ist in der"
Write-Host "README.md beschrieben (Abschnitt 'Windows Schritt fuer Schritt',"
Write-Host "Schritt 1)."
Write-Host ""

$secure = Read-Host "GitHub-Token einfuegen (Eingabe bleibt verborgen)" -AsSecureString
$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
$token = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
[Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)

if ([string]::IsNullOrWhiteSpace($token)) {
    Write-Host ""
    Write-Host "Abgebrochen: leere Eingabe. Kein Eintrag angelegt." -ForegroundColor Yellow
    Write-Host ""
    Read-Host "Beliebige Taste zum Schliessen"
    exit 1
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class GuidanceCredMan
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct CREDENTIAL
    {
        public int Flags;
        public int Type;
        public string TargetName;
        public string Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public int CredentialBlobSize;
        public IntPtr CredentialBlob;
        public int Persist;
        public int AttributeCount;
        public IntPtr Attributes;
        public string TargetAlias;
        public string UserName;
    }

    [DllImport("advapi32.dll", EntryPoint = "CredWriteW", CharSet = CharSet.Unicode)]
    private static extern bool CredWrite(ref CREDENTIAL cred, uint flags);

    // Writes a generic credential whose blob is the secret encoded as UTF-16,
    // which is exactly the encoding the updater's CredRead path expects.
    public static bool Write(string target, string user, string secret)
    {
        IntPtr blob = IntPtr.Zero;
        try
        {
            byte[] bytes = System.Text.Encoding.Unicode.GetBytes(secret);
            blob = Marshal.AllocHGlobal(bytes.Length);
            Marshal.Copy(bytes, 0, blob, bytes.Length);
            CREDENTIAL c = new CREDENTIAL();
            c.Flags = 0;
            c.Type = 1;                       // CRED_TYPE_GENERIC
            c.TargetName = target;
            c.UserName = user;
            c.CredentialBlobSize = bytes.Length;
            c.CredentialBlob = blob;
            c.Persist = 2;                    // CRED_PERSIST_LOCAL_MACHINE
            return CredWrite(ref c, 0);
        }
        finally
        {
            if (blob != IntPtr.Zero) { Marshal.FreeHGlobal(blob); }
        }
    }
}
"@

$ok = [GuidanceCredMan]::Write($target, $env:USERNAME, $token)
if ($ok) {
    Write-Host ""
    Write-Host "Gespeichert: '$target' in deinem Benutzerkonto." -ForegroundColor Green
    Write-Host "Pruefen mit: guidance update check --target guidance"
} else {
    $err = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
    Write-Host ""
    Write-Host "Speichern fehlgeschlagen (Win32-Fehler $err)." -ForegroundColor Red
}

Write-Host ""
Read-Host "Beliebige Taste zum Schliessen"
