# Install claude-usage binary + SessionEnd hook.
# Usage: irm https://raw.githubusercontent.com/OWNER/REPO/main/install.ps1 | iex
#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'  # prevents Invoke-WebRequest from hanging on PS 5.1

$Repo     = "bradmontgomery/claude-usage"
$BinName  = "claude-usage"
$Archive  = "claude-usage-windows-x86_64.zip"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\$BinName"
$ClaudeDir  = Join-Path $env:USERPROFILE ".claude"
$RawBase    = "https://raw.githubusercontent.com/$Repo/main"
$ApiBase    = "https://api.github.com/repos/$Repo"

function Info($msg) { Write-Host "==> $msg" -ForegroundColor Blue }
function Ok($msg)   { Write-Host "  + $msg" -ForegroundColor Green }
function Die($msg)  { Write-Error "Error: $msg"; exit 1 }

# ConvertFrom-Json -AsHashtable requires PS 6+; this works on PS 5.1
function ConvertTo-Hashtable($obj) {
    if ($obj -is [System.Management.Automation.PSCustomObject]) {
        $hash = @{}
        foreach ($prop in $obj.PSObject.Properties) {
            $hash[$prop.Name] = ConvertTo-Hashtable $prop.Value
        }
        return $hash
    } elseif ($obj -is [System.Object[]]) {
        return @($obj | ForEach-Object { ConvertTo-Hashtable $_ })
    }
    return $obj
}

# ── Resolve latest release tag ───────────────────────────────────────────────

Info "Fetching latest release..."
try {
    $release = Invoke-RestMethod "$ApiBase/releases/latest"
} catch {
    Die "Could not fetch latest release: $_"
}
$tag = $release.tag_name
if (-not $tag) { Die "Could not determine latest release tag." }
Ok "Latest release: $tag"

# ── Download & install binary ────────────────────────────────────────────────

$downloadUrl = "https://github.com/$Repo/releases/download/$tag/$Archive"
$tmpDir = Join-Path $env:TEMP "claude-usage-install-$([System.IO.Path]::GetRandomFileName())"
New-Item -ItemType Directory -Path $tmpDir | Out-Null

Info "Downloading $Archive..."
Invoke-WebRequest $downloadUrl -OutFile (Join-Path $tmpDir $Archive)

Info "Installing binary to $InstallDir..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive (Join-Path $tmpDir $Archive) -DestinationPath $tmpDir -Force
Copy-Item (Join-Path $tmpDir "$BinName.exe") (Join-Path $InstallDir "$BinName.exe") -Force
Remove-Item $tmpDir -Recurse -Force
Ok "Installed $InstallDir\$BinName.exe"

# ── Install hook script ──────────────────────────────────────────────────────

Info "Installing SessionEnd hook..."
New-Item -ItemType Directory -Force -Path $ClaudeDir | Out-Null
$hookDest = Join-Path $ClaudeDir "collect-session-stats.py"
Invoke-WebRequest "$RawBase/hook/collect-session-stats.py" -OutFile $hookDest
Ok "Saved hook to $hookDest"

# Register hook in settings.json
$settingsPath = Join-Path $ClaudeDir "settings.json"
$hookCmd = "python3 $hookDest"
$entry = @{
    matcher = ""
    hooks   = @(@{ type = "command"; command = $hookCmd })
}

if (Test-Path $settingsPath) {
    try {
        $config = Get-Content $settingsPath -Raw | ConvertFrom-Json | ConvertTo-Hashtable
    } catch {
        Write-Warning "settings.json contains invalid JSON — skipping hook registration."
        $config = $null
    }
} else {
    $config = @{}
}

if ($config -ne $null) {
    if (-not $config.ContainsKey("hooks"))            { $config["hooks"] = @{} }
    if (-not $config["hooks"].ContainsKey("SessionEnd")) { $config["hooks"]["SessionEnd"] = @() }

    $alreadyRegistered = $config["hooks"]["SessionEnd"] | Where-Object {
        $_.hooks -and $_.hooks[0].command -like "*collect-session-stats.py*"
    }

    if ($alreadyRegistered) {
        Ok "Hook already registered."
    } else {
        $config["hooks"]["SessionEnd"] += $entry
        $json = $config | ConvertTo-Json -Depth 10
        [System.IO.File]::WriteAllText($settingsPath, $json, [System.Text.UTF8Encoding]::new($false))
        Ok "Registered SessionEnd hook in $settingsPath"
    }
}

# ── PATH reminder ────────────────────────────────────────────────────────────

$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$currentPath", "User")
    Ok "Added $InstallDir to your user PATH (restart your terminal to pick it up)"
}

Ok "Done! Run: $BinName --help"
