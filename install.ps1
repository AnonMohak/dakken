# Install dakken on Windows, with no admin rights:
#
#   irm https://raw.githubusercontent.com/AnonMohak/dakken/master/install.ps1 | iex
#
# It downloads the newest dakken.exe from GitHub Releases, checks it against
# the release's SHA256SUMS, puts it in %LOCALAPPDATA%\Programs\dakken, and
# adds that folder to your user PATH. Run it again to update; in dakken, `u`
# in the menu also updates it.
#
# Set DAKKEN_INSTALL_DIR to use another folder. Then the script does not
# change PATH; add the folder yourself.

& {
    $ErrorActionPreference = 'Stop'
    # Windows PowerShell 5.1 needs this for GitHub, and its progress bar
    # makes downloads very slow.
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $ProgressPreference = 'SilentlyContinue'

    $repo = 'AnonMohak/dakken'
    $headers = @{ 'User-Agent' = 'dakken-install' }
    $tag = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers $headers).tag_name
    $name = "dakken-$tag-x86_64-pc-windows-msvc.exe"
    $base = "https://github.com/$repo/releases/download/$tag"

    $dir = $env:DAKKEN_INSTALL_DIR
    $setPath = -not $dir
    if (-not $dir) { $dir = Join-Path $env:LOCALAPPDATA 'Programs\dakken' }
    New-Item -ItemType Directory -Force -Path $dir | Out-Null

    Write-Host "Downloading dakken $tag..."
    $download = Join-Path $dir 'dakken.download'
    $sumsFile = Join-Path $dir 'SHA256SUMS.download'
    try {
        Invoke-WebRequest "$base/$name" -OutFile $download -UseBasicParsing -Headers $headers
        Invoke-WebRequest "$base/SHA256SUMS" -OutFile $sumsFile -UseBasicParsing -Headers $headers
        $want = Get-Content $sumsFile |
            Where-Object { ($_ -split '\s+', 2)[1] -replace '^\*', '' -eq $name } |
            ForEach-Object { ($_ -split '\s+')[0] } |
            Select-Object -First 1
        if (-not $want) { throw "SHA256SUMS has no line for $name." }
        $got = (Get-FileHash $download -Algorithm SHA256).Hash
        if ($got -ne $want) { throw "The download does not match its checksum. Nothing was installed." }
        $exe = Join-Path $dir 'dakken.exe'
        try {
            Move-Item -Force $download $exe
        } catch {
            throw "Cannot replace $exe. Close dakken and run this again."
        }
    } finally {
        Remove-Item -Force -ErrorAction SilentlyContinue $download, $sumsFile
    }

    if ($setPath) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $parts = @($userPath -split ';' | Where-Object { $_ })
        if ($parts -notcontains $dir) {
            [Environment]::SetEnvironmentVariable('Path', (($parts + $dir) -join ';'), 'User')
            Write-Host "Added $dir to your user PATH."
        }
        if (($env:Path -split ';') -notcontains $dir) { $env:Path = "$env:Path;$dir" }
    }

    Write-Host "dakken $tag is installed in $dir."
    if ($setPath) {
        Write-Host 'Run "dakken" to start. (Other open terminals see it after a restart.)'
    } else {
        Write-Host "Add $dir to PATH to run dakken from any folder."
    }
}
