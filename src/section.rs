use lucide_icons::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    Overview,
    SystemJunk,
    Explorer,
    Uninstaller,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::Overview,
        Section::SystemJunk,
        Section::Explorer,
        Section::Uninstaller,
    ];

    pub const CLEANUP: [Section; 3] =
        [Section::SystemJunk, Section::Explorer, Section::Uninstaller];

    pub fn title(self) -> &'static str {
        match self {
            Section::Overview => "Overview",
            Section::SystemJunk => "System Junk",
            Section::Explorer => "Disk Explorer",
            Section::Uninstaller => "Applications",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Section::Overview => Icon::House,
            Section::SystemJunk => Icon::Brush,
            Section::Explorer => Icon::HardDrive,
            Section::Uninstaller => Icon::Package,
        }
    }
}
