use std::path::PathBuf;

use rayon::prelude::*;
use trash::TrashContext;
use trash::macos::{DeleteMethod, TrashContextExtMacos};

#[derive(Debug, Clone, Copy, Default)]
pub struct Cleanup {
    pub trashed: u64,
    pub failed: u64,
}

impl Cleanup {
    pub fn summary(self) -> String {
        if self.failed > 0 {
            format!(
                "Moved {} items to Trash, {} could not be moved",
                self.trashed, self.failed
            )
        } else {
            format!("Moved {} items to Trash", self.trashed)
        }
    }
}

pub fn move_to_trash(paths: &[PathBuf]) -> Cleanup {
    let mut context = TrashContext::new();
    context.set_delete_method(DeleteMethod::NsFileManager);

    let trashed = paths
        .par_iter()
        .filter(|path| context.delete(path).is_ok())
        .count() as u64;

    Cleanup {
        trashed,
        failed: paths.len() as u64 - trashed,
    }
}
