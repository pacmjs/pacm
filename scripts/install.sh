#!/bin/bash

function check_admin {
    if [ "$EUID" -ne 0 ]; then
        echo "This script requires administrator privileges. Please run it as an administrator."
        exit 1
    fi
}

function log_message {
    echo "$1"
}

function show_progress {
    local activity=$1
    local status=$2
    local percent=$3
    echo -ne "$activity: $status ($percent%)\r"
}

check_admin

log_message "Starting installation script..."

repo="pacmjs/pacm"
api_url="https://api.github.com/repos/$repo/releases/latest"
log_message "Fetching latest release information from GitHub..."
response=$(curl -s $api_url)
url=$(echo $response | grep -oP '"browser_download_url": "\K(.*)(?=")' | grep "pacm")

destination_dir="/usr/local/bin/pacm"
destination="$destination_dir/pacm"

log_message "Creating destination directory if it doesn't exist..."
mkdir -p $destination_dir

log_message "Downloading pacm executable from GitHub..."
show_progress "Downloading" "In Progress" 0
curl -L -o $destination $url
show_progress "Downloading" "Completed" 100
echo ""

log_message "Setting permissions for the executable..."
chmod +x $destination

log_message "Adding pacm directory to the system PATH..."
if [[ ":$PATH:" != *":$destination_dir:"* ]]; then
    echo "export PATH=\$PATH:$destination_dir" >> ~/.bashrc
    source ~/.bashrc
fi

log_message "Installation completed. Restart your terminal to use pacm."