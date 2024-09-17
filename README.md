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
There are a couple of different ways:
* It can be placed somewhere in your path and then ran directly from the terminal  
```bash
your-name@hostname: gui-scale
```

* It can be ran from any directory from the terminal
```bash
~/Downloads/gui-scale
```

* It can be ran by clicking the file to open it.
    - ***Note:*** *Just make sure it's been marked as executable first.*

## Are there going to be improvements?
Right now, I only needed the basic functionality that Tailscale provides. However, that doesn't mean that I'm done with it. I will be making enhancements as I get time or if I need it to implement some other Tailscale functionality.  
  
The first feature that comes to mind as an improvement is Taildrop so the tool can be used to send files back and forth on your tailnet.

### To-Do's
* Add the ability to have a dark theme.
* Package as a .deb file.
* Package as an .rpm file.
* Try packaging as a flatpak and putting it into Flathub.
