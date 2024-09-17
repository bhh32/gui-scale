/** 
 * Messages for the GUI to send to perform
 * the actions by Tailscale CLI
*/
#[derive(Debug, Clone, Copy)]
pub enum TailscaleActions {
    Up,
    Down,
    UseSSH(bool),
    AcceptRoutes(bool),
}