#!/usr/bin/bash -e

## Setup
read -s -p "Enter sudo password to place the files in the appropriate places:" pass

## Download the file from GitHub
cd ~/Downloads
wget https://github.com/bhh32/gui-scale/releases/download/0.2.0/gui-scale-0.2.0.tar.gz

## Unpack the tarball
tar xf gui-scale-0.2.0.tar.gz

## Move the unpacked files to their perspective places
sudo -S $pass mv gui-scale /usr/bin/
sudo -S $pass mv gui-scale.desktop /usr/share/applications/
sudo -S $pass mv tailscale-icon.png /usr/share/pixmaps/

# Let the user know it has been installed
echo "GUI-Scale has been installed successfully!"

## Clean up
rm gui-scale-0.2.0.tar.gz bin_install_script.sh
