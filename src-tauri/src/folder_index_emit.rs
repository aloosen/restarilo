use crate::folder_index::FolderChild;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatusKind {
    Idle,
    Running,
    StoppedBounds,
    Failed,
    Success,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderChildDto {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderIndexUpdated {
    pub repo_name: String,
    pub snapshot_id: String,
    pub generation: u64,
    pub parent_path: String,
    pub dirs: Vec<FolderChildDto>,
    pub children_complete: bool,
    pub scan_status: ScanStatusKind,
}

pub fn to_dtos(dirs: &[FolderChild]) -> Vec<FolderChildDto> {
    dirs.iter()
        .map(|d| FolderChildDto {
            name: d.name.clone(),
            path: d.path.clone(),
        })
        .collect()
}
