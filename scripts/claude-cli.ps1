# [INPUT]: 固定 CLI 动作、当前用户根目录与官方原生/npm/bun/WinGet 程序位置。
# [OUTPUT]: 安装来源/版本和真实完成协议；原生安装器下载/解码/解析失败时保留旧程序。
# [POS]: Windows CLI 程序边界；拒绝链接目录、自定义启动器，保留配置、技能、会话和项目。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
param([ValidateSet('read','uninstall','reinstall')][string]$action='read')
$ErrorActionPreference='Stop'
$env:PSModulePath=(Join-Path $PSHOME 'Modules')+';'+$env:PSModulePath
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding
$root=$env:CC_CLI_USER_DIR
if (-not $root) { $root=$env:USERPROFILE }
$root=[IO.Path]::GetFullPath($root)
$package='@anthropic-ai/claude-code'
$native=Join-Path $root '.local\bin\claude.exe'
$versions=Join-Path $root '.local\share\claude\versions'
function Step($message) { Write-Output "CC_CLI_STEP=$message" }
function Fail($message) { throw $message }
function RealPath($path) {
    if (-not (Test-Path -LiteralPath $path)) { return $false }
    $current=Get-Item -LiteralPath $path -Force
    while ($current) {
        if ($current.Attributes -band [IO.FileAttributes]::ReparsePoint) { return $false }
        if ($current -is [IO.FileInfo]) { $current=$current.Directory } else { $current=$current.Parent }
    }
    return $true
}
function Run($program, $arguments) {
    $global:LASTEXITCODE=0
    & $program @arguments 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail 'Claude Code 安装命令执行失败。' }
}
function NativeInstaller {
    try {
        $content=(Invoke-WebRequest -UseBasicParsing -Uri 'https://claude.ai/install.ps1' -TimeoutSec 60).Content
    } catch { Fail 'Claude Code 官方安装器下载失败，请检查网络后重试。' }
    try {
        if (-not $content -or $content.Length -gt 1048576) { throw 'Invalid installer size' }
        # application/octet-stream 响应的 Content 是 byte[]，不能直接交给 ScriptBlock.Create。
        if ($content -is [byte[]]) {
            $content=(New-Object System.Text.UTF8Encoding($false,$true)).GetString($content)
        } elseif ($content -isnot [string]) { throw 'Invalid installer response type' }
        $content=$content.TrimStart([char]0xfeff)
        if ($content -notmatch '^\s*param\s*\(') { throw 'Invalid installer content' }
        return [scriptblock]::Create($content)
    } catch { Fail 'Claude Code 官方安装器内容无效，已停止操作。' }
}
function Status {
    $methods=New-Object 'System.Collections.Generic.List[string]'
    $managed=New-Object 'System.Collections.Generic.List[string]'
    $nativeFiles=@()
    if (RealPath $versions) {
        $nativeFiles=@(Get-ChildItem -LiteralPath $versions -File | Where-Object { $_.Name -match '^\d+\.\d+\.\d+(\.exe)?$' -and (RealPath $_.FullName) })
    }
    if ((RealPath $native) -and $nativeFiles.Count -gt 0) {
        $hash=(Get-FileHash -LiteralPath $native -Algorithm SHA256).Hash
        $matching=@($nativeFiles | Where-Object { (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash -eq $hash })
        if ($matching.Count -gt 0) { $methods.Add('native');$managed.Add($native) }
    }
    $npm=(Get-Command npm.cmd -ErrorAction SilentlyContinue).Source
    $npmRoot=''
    if ($npm) {
        $global:LASTEXITCODE=0
        $npmRoot=((& $npm root -g 2>$null) -join '').Trim()
        if ($LASTEXITCODE -ne 0) { $npmRoot='' }
        $directory=if($npmRoot){Join-Path $npmRoot $package}else{''}
        if ($directory -and (RealPath $directory) -and (Test-Path -LiteralPath (Join-Path $directory 'package.json'))) {
            $metadata=Get-Content -LiteralPath (Join-Path $directory 'package.json') -Raw | ConvertFrom-Json
            if ($metadata.name -eq $package) { $methods.Add('npm') }
        }
    }
    $bun=(Get-Command bun.exe -ErrorAction SilentlyContinue).Source
    $bunRoot=Join-Path $root '.bun\install\global\node_modules'
    if ($bun -and (RealPath (Join-Path $bunRoot $package))) {
        $meta=Join-Path (Join-Path $bunRoot $package) 'package.json'
        if ((Test-Path -LiteralPath $meta) -and ((Get-Content -LiteralPath $meta -Raw | ConvertFrom-Json).name -eq $package)) { $methods.Add('bun') }
    }
    $winget=(Get-Command winget.exe -ErrorAction SilentlyContinue).Source
    $wingetBinary=Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\Anthropic.ClaudeCode_Microsoft.Winget.Source_8wekyb3d8bbwe\claude.exe'
    if($winget -and (RealPath $wingetBinary)) { $methods.Add('winget');$managed.Add($wingetBinary) }
    $cli=''
    $command=Get-Command claude -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { $cli=$command.Source }
    if (-not $cli -and (Test-Path -LiteralPath $native)) { $cli=$native }
    if (-not $cli -and $methods.Contains('winget')) { $cli=$wingetBinary }
    $known=$false
    if ($cli -and (RealPath $cli)) {
        if ($methods.Contains('native') -and $cli -ieq $native) { $known=$true }
        if ($methods.Contains('npm')) {
            $prefix=Split-Path $npmRoot -Parent
            $shim=Join-Path $prefix 'claude.cmd'
            if ($cli -ieq $shim -and (Get-Content -LiteralPath $shim -Raw).Contains('node_modules\@anthropic-ai\claude-code\cli.js')) { $known=$true;$managed.Add($cli) }
        }
        if ($methods.Contains('bun') -and $cli -ieq (Join-Path $root '.bun\bin\claude.exe')) { $known=$true;$managed.Add($cli) }
    }
    if($methods.Contains('winget') -and $cli) {
        $link=Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Links\claude.exe'
        if($cli -ieq $wingetBinary) { $known=$true }
        elseif($cli -ieq $link -and (Test-Path -LiteralPath $link)) {
            $destination=(Get-Item -LiteralPath $link -Force).Target
            if($destination -and [IO.Path]::GetFullPath($destination) -ieq $wingetBinary) { $known=$true;$cli=$wingetBinary }
        }
    }
    $version=''
    if ($known) {
        $env:DISABLE_AUTOUPDATER='1';$global:LASTEXITCODE=0
        $version=((& $cli --version 2>$null) -join '').Trim()
        if ($LASTEXITCODE -ne 0 -or $version -notmatch '^\d+\.\d+\.\d+.*Claude') { Fail 'Claude Code 版本核验失败。' }
    }
    return @{methods=$methods;managed=$managed;nativeFiles=$nativeFiles;npm=$npm;bun=$bun;winget=$winget;cli=$cli;known=$known;version=$version}
}
function RemovePrograms($state) {
    if ($state.methods.Contains('npm')) { Run $state.npm @('uninstall','-g',$package) }
    if ($state.methods.Contains('bun')) { Run $state.bun @('remove','-g',$package) }
    if ($state.methods.Contains('winget')) { Run $state.winget @('uninstall','--id','Anthropic.ClaudeCode','--exact','--silent','--disable-interactivity') }
    if ($state.methods.Contains('native')) {
        Remove-Item -LiteralPath $native -Force
        foreach ($file in $state.nativeFiles) { Remove-Item -LiteralPath $file.FullName -Force }
    }
    foreach ($file in $state.managed) { if (Test-Path -LiteralPath $file) { Fail 'Claude Code 卸载未完成，部分程序仍存在。' } }
}
try {
    $state=Status
    if ($action -ne 'read') {
        if ($state.cli -and -not $state.known) { Fail '检测到自定义 Claude 启动器，未删除或重装。' }
        $method=if($state.methods.Count){$state.methods[0]}else{'native'}
        $installer=$null
        if ($action -eq 'reinstall' -and $method -eq 'native') {
            Step '正在下载 Claude Code 官方安装器。'
            # 先下载并检查脚本，再卸载；网络或响应异常时不丢失已有程序。
            $installer=NativeInstaller
        }
        Step '正在卸载 Claude Code CLI。'
        RemovePrograms $state
        if ($action -eq 'reinstall') {
            Step '正在重装 Claude Code CLI。'
            if ($method -eq 'npm') { Run $state.npm @('install','-g',"$package@latest") }
            elseif ($method -eq 'winget') { Run $state.winget @('install','--id','Anthropic.ClaudeCode','--exact','--source','winget','--scope','user','--silent','--accept-package-agreements','--accept-source-agreements','--disable-interactivity') }
            elseif ($method -eq 'bun') { Run $state.bun @('install','-g',"$package@latest") }
            else {
                Step '正在使用官方 Claude Code 安装器。'
                try {
                    $global:LASTEXITCODE=0
                    & $installer | Out-Null
                    if ($LASTEXITCODE -ne 0) { throw 'Native installer failed' }
                } catch { Fail 'Claude Code 原生重装失败，请检查网络和安装目录权限。' }
            }
            Step '正在核验 Claude Code CLI 版本。'
            $state=Status
            if (-not $state.known -or -not $state.version) { Fail 'Claude Code 重装后的版本核验失败。' }
        } else { $state=Status;if($state.methods.Count){Fail 'Claude Code 卸载未完成。'} }
    }
    Write-Output "CC_CLI_METHOD=$($state.methods -join ' / ')"
    Write-Output "CC_CLI_PATH=$($state.cli)"
    Write-Output "CC_CLI_VERSION=$($state.version)"
    Write-Output 'CC_CLI_DONE=1'
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
