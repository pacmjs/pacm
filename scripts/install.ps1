function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Write-Message {
    param (
        [string]$message
    )
    Write-Host $message
}

function Show-ProgressBar {
    param (
        [string]$activity,
        [string]$status,
        [int]$percentComplete
    )
    Write-Progress -Activity $activity -Status $status -PercentComplete $percentComplete
}

if (-not (Test-Administrator)) {
    Write-Message "This script requires administrator privileges. Please run it as an administrator."
    Start-Process powershell.exe "-File $PSCommandPath" -Verb RunAs
    exit
}

Write-Message "Starting installation script..."

$repo = "pacmjs/pacm"
$apiUrl = "https://api.github.com/repos/$repo/releases/latest"
Write-Message "Fetching latest release information from GitHub..."
$response = Invoke-RestMethod -Uri $apiUrl
$asset = $response.assets | Where-Object { $_.name -eq "pacm.exe" }
$url = $asset.browser_download_url

$destinationDir = "C:\Program Files\pacm"
$destination = "$destinationDir\pacm.exe"

Write-Message "Creating destination directory if it doesn't exist..."
if (-not (Test-Path -Path $destinationDir)) {
    New-Item -ItemType Directory -Path $destinationDir
}

Write-Message "Downloading pacm executable from github..."
Show-ProgressBar -Activity "Downloading" -Status "In Progress" -PercentComplete 0
Invoke-WebRequest -Uri $url -OutFile $destination
Show-ProgressBar -Activity "Downloading" -Status "Completed" -PercentComplete 100

Write-Message "Setting permissions for the executable..."
icacls $destination /grant Everyone:F

Write-Message "Adding pacm directory to the system PATH..."
$envPath = [System.Environment]::GetEnvironmentVariable("Path", [System.EnvironmentVariableTarget]::Machine)
if ($envPath -notlike "*$destinationDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$envPath;$destinationDir", [System.EnvironmentVariableTarget]::Machine)
}

Write-Message "Installation completed. Restart your terminal to use pacm."