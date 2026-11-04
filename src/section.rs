use lucide_icons::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    SmartScan,
    SystemJunk,
    LargeFiles,
    Uninstaller,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::SmartScan,
        Section::SystemJunk,
        Section::LargeFiles,
        Section::Uninstaller,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::SmartScan => "Smart Scan",
            Section::SystemJunk => "System Junk",
            Section::LargeFiles => "Large Files",
            Section::Uninstaller => "Uninstaller",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Section::SmartScan => Icon::Sparkles,
            Section::SystemJunk => Icon::Brush,
            Section::LargeFiles => Icon::FileStack,
            Section::Uninstaller => Icon::PackageX,
        }
    }
}
