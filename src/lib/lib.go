package lib

import (
	"errors"
	"fmt"
	"io"
	"os/exec"
	"strings"
)

func GetTailscaleIp() (string, error) {
	ipCmd := exec.Command("tailscale", "ip", "-4")
	cmdOutput, err := ipCmd.Output()

	if err != nil {
		return "", fmt.Errorf("couldn't get Tailscale IPv4 address!\n%s", err)
	}

	return string(cmdOutput), nil
}

func GetTailscaleConStatus() (bool, error) {
	// Setup the Tailscale command
	conCmd := exec.Command("tailscale", "debug", "prefs")
	// Setup the grep command to read the ouput of the Tailscale command
	grepCmd := exec.Command("grep", "WantRunning")

	// Setup the pipe for the two commands to be connected
	pipeRead, pipeWrite := io.Pipe()

	// Connect the connection command's stdout to the pipe writer
	conCmd.Stdout = pipeWrite
	// Connect the grep command's stdin to the pipe reader
	grepCmd.Stdin = pipeRead

	// Start the connection command
	conCmd.Start()

	// Defer the pipe reader to be closed at the end of the function
	defer pipeRead.Close()

	// Run a go routine to close the pipe writer and have the
	// connection command wait to be completed until the grep
	// command has been started.
	go func() {
		defer pipeWrite.Close()
		conCmd.Wait()
	}()

	// Have the grep command wait to get its piped input
	grepCmd.Wait()
	// Get the grep command's output
	grepOutput, err := grepCmd.Output()

	// Make sure there wasn't an error, if there was return false
	// and the error message.
	if err != nil {
		return false, err
	}

	// Check if the grep output contains true for the
	// setting grep is checking.
	// If it's true, return true and no error
	if strings.Contains(string(grepOutput), "true") {
		return true, nil
	}

	// Otherwise, return false and no error.
	return false, nil
}

func GetTailscaleSshStatus() (bool, error) {
	sshCmd := exec.Command("tailscale", "debug", "prefs")
	grepCmd := exec.Command("grep", "RunSSH")

	// Setup the pipe for the two commands to be connected
	pipeRead, pipeWrite := io.Pipe()

	// Connect the ssh command's stdout to the pipe writer
	sshCmd.Stdout = pipeWrite
	// Connect the grep command's stdin to the pipe reader
	grepCmd.Stdin = pipeRead

	sshCmd.Start()

	defer pipeRead.Close()

	go func() {
		defer pipeWrite.Close()
		sshCmd.Wait()
	}()

	grepCmd.Wait()
	grepOutput, err := grepCmd.Output()

	// Make sure there wasn't an error, if there was return false
	// and the error message.
	if err != nil {
		return false, err
	}

	// Check if the grep output contains true for the
	// setting grep is checking.
	// If it's true, return true and no error
	if strings.Contains(string(grepOutput), "true") {
		return true, nil
	}

	// Otherwise, return false and no error.
	return false, nil
}

func GetTailscaleRoutesStatus() (bool, error) {
	routesCmd := exec.Command("tailscale", "debug", "prefs")
	grepCmd := exec.Command("grep", "RouteAll")

	// Setup the pipe for the two commands to be connected
	pipeRead, pipeWrite := io.Pipe()

	// Connect the ssh command's stdout to the pipe writer
	routesCmd.Stdout = pipeWrite
	// Connect the grep command's stdin to the pipe reader
	grepCmd.Stdin = pipeRead

	routesCmd.Start()

	defer pipeRead.Close()

	go func() {
		defer pipeWrite.Close()
		routesCmd.Wait()
	}()

	grepCmd.Wait()
	grepOutput, err := grepCmd.Output()

	// Make sure there wasn't an error, if there was return false
	// and the error message.
	if err != nil {
		return false, err
	}

	// Check if the grep output contains true for the
	// setting grep is checking.
	// If it's true, return true and no error
	if strings.Contains(string(grepOutput), "true") {
		return true, nil
	}

	// Otherwise, return false and no error.
	return false, nil
}

func TailscaleConUpDown(upDown bool) error {
	var conCmd *exec.Cmd
	var errMsg error
	if upDown {
		conCmd = exec.Command("tailscale", "up")
		errMsg = fmt.Errorf("tailscale could not be connected")
	} else {
		conCmd = exec.Command("tailscale", "down")
		errMsg = fmt.Errorf("tailscale could not be disconnected")
	}

	_, err := conCmd.Output()

	if err != nil {
		return fmt.Errorf("%s\n%s", errMsg.Error(), err)
	}

	return nil
}

func SetSsh(ssh bool) error {
	var sshCmd *exec.Cmd
	var errMsg error

	if ssh {
		sshCmd = exec.Command("tailscale", "set", "--ssh")
		errMsg = fmt.Errorf("tailscale couldn't enable ssh")
	} else {
		sshCmd = exec.Command("tailscale", "set", "--ssh=false")
		errMsg = fmt.Errorf("tailscale couldn't disable ssh")
	}

	_, err := sshCmd.Output()

	if err != nil {
		return fmt.Errorf("%s\n%s", errMsg.Error(), err)
	}

	return nil
}

func SetRoutes(acceptRoutes bool) error {
	var routesCmd *exec.Cmd
	var errMsg error

	if acceptRoutes {
		routesCmd = exec.Command("tailscale", "set", "--accept-routes")
		errMsg = fmt.Errorf("tailscale couldn't enable accepting routes")
	} else {
		routesCmd = exec.Command("tailscale", "set", "--accept-routes=false")
		errMsg = fmt.Errorf("tailscale couldn't disable accepting routes")
	}

	_, err := routesCmd.Output()

	if err != nil {
		return fmt.Errorf("%s\n%s", errMsg.Error(), err)
	}

	return nil
}

// Tail Drop Section

// Send files through Tail Drop
func TailscaleSend(filePath string, target string) (string, error) {
	errMsg := errors.New("")
	status := ""
	go func() {
		sendCmd := exec.Command("tailscale", "file", "cp", filePath, getFormattedTarget(target))
		_, err := sendCmd.Output()

		if err != nil {
			errMsg = fmt.Errorf("tailscale send for %s failed", filePath)
		} else {
			errMsg = nil
			status = "Successfully sent " + filePath + " to " + target + "!"
		}

	}()

	return status, errMsg
}

// Receive files through Tail Drop
func TaislcaleReceive() string {
	userNameCmd := exec.Command("whoami")
	userNameOut, _ := userNameCmd.Output()
	userName := string(userNameOut)
	downloadPath := fmt.Sprintf("/home/%s/Downloads/", userName)
	downloadPath = strings.Replace(downloadPath, "\n", "", -1)
	statusMsg := ""

	go func() {
		rxCmd := exec.Command("tailscale", "file", "get", downloadPath)
		_, err := rxCmd.Output()

		if err != nil {
			statusMsg = "Failed to get files"
		}
	}()

	if !strings.Contains(statusMsg, "Failed") {
		statusMsg = "Receiving Files"
	}

	return statusMsg
}

func GetTailscaleDevices() []string {
	statusCmd := exec.Command("tailscale", "status")
	statusOutput, _ := statusCmd.Output()

	lines := strings.Split(string(statusOutput), "\n")
	retStr := make([]string, len(lines))

	for idx, line := range lines {
		if idx == 0 {
			continue
		}
		ele := strings.Fields(line)
		if len(ele) > 1 {
			retStr[idx] = ele[1]
		}
	}

	return retStr
}

// Helper functions
func getFormattedTarget(target string) string {
	return target + ":"
}
