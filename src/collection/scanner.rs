use std::path::Path;

use crate::collection::model::{Collection, CollectionFormat};

pub fn scan_collection(root: &Path, format: CollectionFormat) -> Collection {
    Collection {
        root: root.to_path_buf(),
        format,
        nodes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::collection::model::CollectionFormat;

    use super::scan_collection;

    #[test]
    fn scan_collection_builds_empty_placeholder_model() {
        let dir = tempdir().expect("temp dir");

        let collection = scan_collection(dir.path(), CollectionFormat::ClassicJson);

        assert_eq!(collection.root, dir.path());
        assert!(collection.nodes.is_empty());
    }
}
