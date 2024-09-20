# About
Tailscale CLI GUI wrapper built for Linux.

## Why create this?
I started working on this because I have family members that I administer their computers for and I wanted an easy way to have them to connect to Tailscale when I need to do some maintenance operations. Tailscale doesn't have a GUI for Linux, so I needed to roll my own.

## Dependencies
You must have Tailscale already installed on you computer and be running it in user mode. I did not want to make a tool that needed sudo passwords to run the commands since my family members do not have sudo privileges.

```bash
sudo tailscale set --operator=$USER
```

## How does it work?
It uses the Tailscale CLI behind the scenes to do everything, we just don't have to type the commands by hand.

## How do I use it?
### The Binary File
The best way to use this file is to use the install script in the releases section. It will "install" the binary and associated icon and .desktop files. These are all included in the tarball (.tar.gz) file in the releases section. No need to download the tarball if using the install script. It will download it for you, and then clean up after itself so you don't have to.  
    
- **Note**: It will ask you for your sudo password. This is just because it needs access to place the .desktop and icon file in the perspective places as if there is an install happening.  

Once the install script is downloaded, you can run the following commands from the terminal:

```bash
cd ~/Downloads
chmod +x bin_install_script.sh
./bin_install_script.sh
```

### The .deb File
This file is used for Debian/Ubuntu derivative distros (i.e.; Debian, Ubuntu, Kubuntu, Pop!OS, Mint, etc.). There are two ways to install this package:

#### Terminal
Open a terminal and navigate to the directory it was downloaded to:
```bash
cd ~/Downloads
```

Then, use the following command to install it:
```bash
sudo apt install -y ./gui-scale.deb
```

Once it's done installing, you can just navigate to it like any other program.

#### Double-click
Navigate to the file in your file manager.
Double-click it and it should open a program such as Gdebi to install it.  
**Note**: If it doesn't open a program, such as GDebi, to install the package. You will first have to install GDebi and then install this package. To install GDebi, navigate to your app store, search for it, and then install.  

&nbsp;  
Once it's installed, you can just navigate to it like any other program.

### The .rpm File
This file is used for Fedora/RHEL derivative distros (i.e.; Fedora, RHEL, CentOS Stream, Alma Linux, Bluefin, Aurora, etc.). There are two ways you can install it, but the terminal way is going to have a slightly different command if you're on something like  Fedora Silverblue, Kinoite, Bluefin, or Aurora.

#### Terminal Non-atomic
Open a terminal and navigate to the directory it was downloaded to:
```bash
cd ~/Downloads
```

Then, use the following command to install it:
```bash
sudo dnf install -y ./gui-scale-0.2.0-1.fc40.x86_64.rpm
```

Once it's done installing, you can open and run it just like any other program installed to your computer.


#### Terminal Atomic (Silverblue, Kinoite, Bluefin, Aurora...)
Open a terminal and navigate to the directory it was downloaded to:
```bash
cd ~/Downloads
```

Then, use the following command to install it:
```bash
rpm-ostree install ./gui-scale-0.2.0-1.fc40.x86_64.rpm
```

When it's done installing run this command to reboot your computer so the change can take effect:
```bash
systemctl reboot
```

Once your computer reboots and you're logged back in, you can open and run it just like any other program installed on your computer.

## Are there going to be improvements?
Right now, I only needed the basic functionality that Tailscale provides. However, that doesn't mean that I'm done with it. I will be making enhancements as I get time or if I need it to implement some other Tailscale functionality.  
  
The first feature that comes to mind as an improvement is Taildrop so the tool can be used to send files back and forth on your tailnet.

### To-Do's
* Add the ability to have a dark theme.
* Add TailDrop functionality.

### Possible Future Features
* Nix package
    - I've never created a Nix package before, so I would have to learn this. Currently, I don't have the time to learn it, but, because of the popularity, I do plan on eventually getting around to it.

### Can't Currently Do's
* Create a flatpak.
    - Because of the nature of using Tailscale CLI under the hood and flatpak's sandboxing, I would have to have a completely different version with sandbox breakout commands to get it to work correctly.
    - This would take more time than I currently have, but it is not off the table for me to redo some things to make a flatpak version.
* Create a Snap package.
    - I don't understand how snaps work enough to know if I would need to create a different version like I do with flatpak. Again, not off the table, but not something that I currently plan on doing.
