package app

import (
	"fmt"
	"os"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/bhh32/gui-scale/v2/src/lib"
)

func App() {
	// Create a new application
	a := app.New()
	// Create the main window
	window := a.NewWindow("GUI Scale")
	window.Resize(fyne.NewSize(640.0, 250.0))
	window.SetFixedSize(true)

	// Create a container for the widgets
	pageContainer := container.NewAppTabs()
	pageContainer.SetTabLocation(container.TabLocationTop)

	// Setup the basic functionality page
	setupBasicPage(pageContainer)

	// Setup the tail drop functionality page
	setupTailDropPage(pageContainer, &window)

	// Setup the exit node functionality page
	setupExitNodePage(pageContainer)

	window.SetContent(pageContainer)

	window.ShowAndRun()
}

func setupBasicPage(pageContainer *container.AppTabs) {
	basicPageContainer := container.NewVBox()
	basicPageTabContainer := container.NewTabItemWithIcon("Basic", theme.HomeIcon(), basicPageContainer)
	pageContainer.Append(basicPageTabContainer)

	// Get the Tailscale IPv4 address
	ipAddrStr, err := lib.GetTailscaleIp()
	// Check to see if there was an error gettin the IPv4 address
	if err != nil {
		// Print the error to the terminal
		// Note this will be changed to log for production
		fmt.Fprintf(os.Stderr, "%s\n", err)
	}

	// Set the label to the address
	ipAddrLabel := widget.NewLabel("Tailscale IPv4 Address: " + ipAddrStr)

	ipAddrContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), ipAddrLabel, layout.NewSpacer())
	basicPageContainer.Add(ipAddrContainer)

	// Get the Tailscale connection status
	isConnected, err := lib.GetTailscaleConStatus()

	// Create a connection status label and add it to the header container
	isConnectedLabel := widget.NewLabel("")

	// Create a connected disconnect status container
	connectionStatusContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), widget.NewLabel("Connection Status:"), isConnectedLabel, layout.NewSpacer())
	basicPageContainer.Add(connectionStatusContainer)

	// Create a variable to hold the connection check label
	connectDisconnectLabel := widget.NewLabel("")
	if isConnected {
		connectDisconnectLabel.SetText("Disconnect")
		isConnectedLabel.SetText("Connected")
	} else {
		connectDisconnectLabel.SetText("Connect")
		isConnectedLabel.SetText("Not Connected")

	}

	// Create the Connect/Disconnect check
	connectDisconnectCheck := widget.NewCheck("", func(isCon bool) {
		go func() {
			if isCon {
				connectDisconnectLabel.SetText("Disconnect")
				isConnectedLabel.SetText("Connected")
			} else {
				connectDisconnectLabel.SetText("Connect")
				isConnectedLabel.SetText("Not Connected")
			}
		}()

		go func() {
			err := lib.TailscaleConUpDown(isCon)

			if err != nil {
				fmt.Fprintf(os.Stderr, "%s\n", err)
			}
		}()

		isConnected = isCon
	})

	// Check to see if is connected was true
	if isConnected {
		// Set the label text to connected
		isConnectedLabel.SetText("Connected")
		connectDisconnectCheck.SetChecked(true)
	} else {
		// Check to see if there was en error getting the connection status
		if err != nil {
			// Print the error to stderr
			// Note: This will be moved to log for production
			fmt.Fprintf(os.Stderr, "%s/n", err)
			// Set the text of the lable to Error: Status unknown
			isConnectedLabel.SetText("Error: Status Unknown")
		} else {
			// There was no error and Tailscale just isn't connected.
			// Set the label text to Not Connected.
			isConnectedLabel.SetText("Not Connected")
		}
		connectDisconnectCheck.SetChecked(false)
	}

	connectionContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), connectDisconnectLabel, connectDisconnectCheck, layout.NewSpacer())
	basicPageContainer.Add(connectionContainer)

	// Get if Tailscale SSH is enabled or not on initial load
	sshEnabled, _ := lib.GetTailscaleSshStatus()

	// Create a ssh status label
	sshEnabledLabel := widget.NewLabel("SSH Enabled")

	// Create a ssh status check
	sshEnabledCheck := widget.NewCheck("", func(isEnabled bool) {
		if isEnabled {
			sshEnabledLabel.SetText("SSH Enabled")
		} else {
			sshEnabledLabel.SetText("SSH Disabled")
		}
		go func() {
			err := lib.SetSsh(isEnabled)
			if err != nil {
				fmt.Fprintf(os.Stderr, "%s\n", err)
			}
		}()
		sshEnabled = isEnabled
	})

	// Add sshEnabledCheck to the container
	sshContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), sshEnabledLabel, sshEnabledCheck, layout.NewSpacer())
	basicPageContainer.Add(sshContainer)

	if sshEnabled {
		// Set the check to enabled
		sshEnabledCheck.SetChecked(true)
	} else {
		sshEnabledCheck.SetChecked(false)
	}

	// Get if Tailscale Accept Routes is enabled or not on initial load
	routesAccepted, _ := lib.GetTailscaleRoutesStatus()

	// Create an accept routes status label
	acceptRoutesLabel := widget.NewLabel("Accept Routes")

	// Create an accept routes status check
	acceptRoutesCheck := widget.NewCheck("", func(isEnabled bool) {
		if isEnabled {
			acceptRoutesLabel.SetText("Routes Accepted")
		} else {
			acceptRoutesLabel.SetText("Accept Routes")
		}

		err := lib.SetRoutes(isEnabled)
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s\n", err)
		}
		routesAccepted = isEnabled
	})

	if routesAccepted {
		acceptRoutesCheck.SetChecked(true)
	} else {
		acceptRoutesCheck.SetChecked(false)
	}
	routesContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), acceptRoutesLabel, acceptRoutesCheck, layout.NewSpacer())
	basicPageContainer.Add(routesContainer)
}

func setupTailDropPage(pageContainer *container.AppTabs, mainWindow *fyne.Window) {
	tailDropPageContainer := container.NewVBox()

	tailDropTab := container.NewTabItemWithIcon("Tail Drop", theme.MailSendIcon(), tailDropPageContainer)

	devices := lib.GetTailscaleDevices()
	selectedDev := ""
	deviceSel := widget.NewSelect(devices, func(device string) {
		selectedDev += device
	})

	selectedFileLabel := widget.NewLabel("")
	filePath := ""

	filePickerBtn := widget.NewButtonWithIcon("Choose File", theme.FileIcon(), func() { showFileDialog(*mainWindow, selectedFileLabel, &filePath) })
	sendFileBtn := widget.NewButtonWithIcon("Send File", theme.MailSendIcon(), func() {
		go func() {
			status := "Sending file..."
			selectedFileLabel.SetText(status)
			lib.TailscaleSend(filePath, selectedDev)

			selectedFileLabel.SetText(status)
			selectedDev = ""
			deviceSel.SetSelected("")

			go func() {
				time.Sleep(5 * time.Second)

				selectedFileLabel.SetText("File sent!")
				status = ""
				time.Sleep(4 * time.Second)
				selectedFileLabel.SetText(status)
			}()
		}()

	})

	receiveFileLabel := widget.NewLabel("Click `Receive All Files` to get your files...")
	downloadLocationLabel := widget.NewLabel("Files will be downloaded to your Downloads directory")
	deviceSelContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), deviceSel, filePickerBtn, layout.NewSpacer())
	fileSelectionContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), layout.NewSpacer(), selectedFileLabel, layout.NewSpacer(), layout.NewSpacer())
	sendFileContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), layout.NewSpacer(), sendFileBtn, layout.NewSpacer(), layout.NewSpacer())
	receiveFileContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), layout.NewSpacer(), widget.NewButtonWithIcon("Receive All Files", theme.MailAttachmentIcon(), func() {
		go func() {
			status := lib.TaislcaleReceive()
			receiveFileLabel.SetText(status)
			for i := 0; i < 10; i++ {
				time.Sleep(1 * time.Second)

				status = status + "."

				receiveFileLabel.SetText(status)
			}
			receiveFileLabel.SetText("Files received!")
			time.Sleep(5 * time.Second)
			receiveFileLabel.SetText("Click `Receive All Files` to get your files...")
		}()
	}), layout.NewSpacer(), layout.NewSpacer())
	receiveFileStatusContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), layout.NewSpacer(), receiveFileLabel, layout.NewSpacer(), layout.NewSpacer())
	downloadLocationContainer := container.New(layout.NewHBoxLayout(), layout.NewSpacer(), layout.NewSpacer(), downloadLocationLabel, layout.NewSpacer(), layout.NewSpacer())

	tailDropPageContainer.Add(deviceSelContainer)
	tailDropPageContainer.Add(fileSelectionContainer)
	tailDropPageContainer.Add(sendFileContainer)
	tailDropPageContainer.Add(layout.NewSpacer())
	tailDropPageContainer.Add(widget.NewSeparator())
	tailDropPageContainer.Add(layout.NewSpacer())
	tailDropPageContainer.Add(receiveFileContainer)
	tailDropPageContainer.Add(receiveFileStatusContainer)
	tailDropPageContainer.Add(downloadLocationContainer)
	pageContainer.Append(tailDropTab)
}

func setupExitNodePage(pageContainer *container.AppTabs) {
	exitNodePageContainer := container.NewVBox()
	exitNodeTab := container.NewTabItemWithIcon("Exit Node(s)", theme.ComputerIcon(), exitNodePageContainer)

	pageContainer.Append(exitNodeTab)
}

// Helper Functions
func showFileDialog(window fyne.Window, selectedFileLabel *widget.Label, filePath *string) {

	dialog.ShowFileOpen(func(file fyne.URIReadCloser, err error) {
		if err != nil {
			dialog.ShowError(err, window)
			return
		}

		if file == nil {
			return
		}

		selectedFileLabel.SetText(file.URI().Path())
		*filePath = file.URI().Path()
	}, window)
}
