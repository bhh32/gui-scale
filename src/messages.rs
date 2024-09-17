#[derive(Debug, Clone, Copy)]
pub enum TailscaleActions {
    Up,
    Down,
    UseSSH(bool),
    AcceptRoutes(bool),
}