# PowerShell script to download and install pacm on Windows

# Define the URL to download the pacm executable
$url = "https://example.com/path/to/pacm.exe"

# Define the destination path for the downloaded executable
$destination = "$env:USERPROFILE\Downloads\pacm.exe"

# Download the pacm executable
Invoke-WebRequest -Uri $url -OutFile $destination

# Make the downloaded executable file executable
icacls $destination /grant Everyone:F

# Run the pacm executable
Start-Process $destination
