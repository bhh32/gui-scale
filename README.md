# About
The reason I am creating this application to better help my in-laws with their computer. I use Tailscale to be able to remotely update, troubleshoot, and generally just help them with their computers. They are on Linux Mint, so to help me help them, I'm creating this application so they can know if Tailscale is connected and the settings are enabled that I need them to enable.  
  
## Features
Although my in-laws only need the basics tab, I am implementing the Tail Drop and exit node functionality so it's useful to more people.  
  
## History
Originally, this was a Rust application, but I have limited time to implement the functionality and the Iced GUI framework was proving to be too unstable. So, I have started the conversion to Go using the Fyne GUI framework.  
  
## Related Applications
This is the big brother application to the [GUI Scale Applet](https://github.com/cosmic-utils/gui-scale-applet) that is built for System76's COSMIC desktop. Both will have the same functionality when I'm done, but the applet is ahead in its development since it's using Libcosmic over iced.  
  
I am the creator and maintainer of both the applet and this application.