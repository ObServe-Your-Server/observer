#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationKind {
    Cpu,
    Memory,
    Disk,
    ContainerSocket,
}