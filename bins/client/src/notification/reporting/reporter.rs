

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum Reporter {
    System,
    Cpu,
    Memory,
    Disk,
    ContainerSocket,
}