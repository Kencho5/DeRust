#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    SmartScan,
    SystemJunk,
    LargeFiles,
    Uninstaller,
    Malware,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::SmartScan,
        Section::SystemJunk,
        Section::LargeFiles,
        Section::Uninstaller,
        Section::Malware,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::SmartScan => "Smart Scan",
            Section::SystemJunk => "System Junk",
            Section::LargeFiles => "Large Files",
            Section::Uninstaller => "Uninstaller",
            Section::Malware => "Malware",
        }
    }
}
