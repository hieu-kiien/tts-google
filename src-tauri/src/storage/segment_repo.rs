use crate::storage::db::DatabaseManager;
use crate::storage::project_repo::ProjectRepository;

pub struct SegmentRepository;

impl SegmentRepository {
    pub fn update_text(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
        text: &str,
        prompt: &str,
        fingerprint: &str,
    ) -> Result<(), String> {
        ProjectRepository::update_segment_text(
            db,
            project_id,
            segment_id,
            text,
            prompt,
            fingerprint,
        )
    }

    pub fn split_segment(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
        split_index: usize,
    ) -> Result<(), String> {
        ProjectRepository::split_segment(db, project_id, segment_id, split_index)
    }

    pub fn merge_with_previous(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
    ) -> Result<(), String> {
        ProjectRepository::merge_segment_with_previous(db, project_id, segment_id)
    }
}
