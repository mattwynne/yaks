/// Contract tests that must pass for all ReadYakStore + WriteYakStore implementations.
/// Use the yak_store_tests! macro to run against any implementation.
///
/// The macro accepts an expression that returns `(impl ReadYakStore + WriteYakStore, _guard)`.
/// The `_guard` keeps any resources (like TempDir) alive for the test duration.
/// For implementations that don't need a guard, pass `()`.
macro_rules! yak_store_tests {
    ($create_store:expr) => {
        use crate::domain::ports::{ReadYakStore, WriteYakStore};
        use crate::domain::{CONTEXT_FIELD, STATE_FIELD};

        // --- WriteYakStore ---

        #[test]
        fn create_yak_is_retrievable() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let yak = ReadYakStore::get_yak(&store, "test-yak").unwrap();
            assert_eq!(yak.name, "test-yak");
        }

        #[test]
        fn create_duplicate_yak_errors() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let result = store.create_yak("test-yak", "", None);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("already exists"));
        }

        #[test]
        fn delete_yak_removes_it() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            store.delete_yak("test-yak").unwrap();
            assert!(!ReadYakStore::yak_exists(&store, "test-yak"));
        }

        #[test]
        fn delete_nonexistent_yak_succeeds() {
            let (store, _guard) = $create_store;
            let result = store.delete_yak("nonexistent");
            assert!(result.is_ok());
        }

        #[test]
        fn rename_yak_moves_with_fields() {
            let (store, _guard) = $create_store;
            store.create_yak("old-name", "", None).unwrap();
            store
                .write_field("old-name", CONTEXT_FIELD, "Context text")
                .unwrap();
            store.write_field("old-name", STATE_FIELD, "done").unwrap();

            store.rename_yak("old-name", "new-name").unwrap();

            let result = ReadYakStore::get_yak(&store, "old-name");
            assert!(result.is_err());

            let yak = ReadYakStore::get_yak(&store, "new-name").unwrap();
            assert_eq!(yak.name, "new-name");
            assert!(yak.is_done());
            assert_eq!(yak.context.unwrap(), "Context text");
        }

        #[test]
        fn rename_nonexistent_yak_errors() {
            let (store, _guard) = $create_store;
            let result = store.rename_yak("nonexistent", "new-name");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn rename_to_existing_yak_errors() {
            let (store, _guard) = $create_store;
            store.create_yak("yak1", "", None).unwrap();
            store.create_yak("yak2", "", None).unwrap();
            let result = store.rename_yak("yak1", "yak2");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("already exists"));
        }

        #[test]
        fn write_field_is_readable() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            store
                .write_field("test-yak", "notes", "Field content")
                .unwrap();
            let content = ReadYakStore::read_field(&store, "test-yak", "notes").unwrap();
            assert_eq!(content, "Field content");
        }

        #[test]
        fn write_field_with_dots_in_name() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            store
                .write_field("test-yak", "notes.txt", "Text file")
                .unwrap();
            let content = ReadYakStore::read_field(&store, "test-yak", "notes.txt").unwrap();
            assert_eq!(content, "Text file");
        }

        #[test]
        fn write_field_nonexistent_yak_errors() {
            let (store, _guard) = $create_store;
            let result = store.write_field("nonexistent", "field", "content");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        // --- ReadYakStore ---

        #[test]
        fn get_yak_defaults() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let yak = ReadYakStore::get_yak(&store, "test-yak").unwrap();
            assert_eq!(yak.state, "todo");
            assert_eq!(yak.context, None);
            assert!(!yak.is_done());
        }

        #[test]
        fn get_nonexistent_yak_errors() {
            let (store, _guard) = $create_store;
            let result = ReadYakStore::get_yak(&store, "nonexistent");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn list_yaks_returns_all() {
            let (store, _guard) = $create_store;
            store.create_yak("yak1", "", None).unwrap();
            store.create_yak("yak2", "", None).unwrap();
            let yaks = ReadYakStore::list_yaks(&store).unwrap();
            assert_eq!(yaks.len(), 2);
        }

        #[test]
        fn list_yaks_empty() {
            let (store, _guard) = $create_store;
            let yaks = ReadYakStore::list_yaks(&store).unwrap();
            assert_eq!(yaks.len(), 0);
        }

        #[test]
        fn yak_exists_returns_correct_value() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            assert!(ReadYakStore::yak_exists(&store, "test-yak"));
            assert!(!ReadYakStore::yak_exists(&store, "missing"));
        }

        #[test]
        fn find_yak_exact_match() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let result = ReadYakStore::find_yak(&store, "test-yak").unwrap();
            assert_eq!(result, "test-yak");
        }

        #[test]
        fn find_yak_fuzzy_match() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let result = ReadYakStore::find_yak(&store, "test").unwrap();
            assert_eq!(result, "test-yak");
        }

        #[test]
        fn find_yak_case_insensitive() {
            let (store, _guard) = $create_store;
            store.create_yak("Fix the Bug", "", None).unwrap();
            let result = ReadYakStore::find_yak(&store, "the bug").unwrap();
            assert_eq!(result, "Fix the Bug");
        }

        #[test]
        fn find_yak_matches_leaf_not_full_path() {
            let (store, _guard) = $create_store;
            store.create_yak("parent", "", None).unwrap();
            store.create_yak("parent/child1", "", None).unwrap();

            let result = ReadYakStore::find_yak(&store, "parent").unwrap();
            assert_eq!(result, "parent");

            let result = ReadYakStore::find_yak(&store, "child1").unwrap();
            assert_eq!(result, "parent/child1");
        }

        #[test]
        fn find_yak_leaf_only_no_ambiguity() {
            let (store, _guard) = $create_store;
            store.create_yak("parent/child1", "", None).unwrap();

            let result = ReadYakStore::find_yak(&store, "parent");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn find_yak_ambiguous_errors() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak1", "", None).unwrap();
            store.create_yak("test-yak2", "", None).unwrap();
            let result = ReadYakStore::find_yak(&store, "test");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("ambiguous"));
        }

        #[test]
        fn find_yak_not_found_errors() {
            let (store, _guard) = $create_store;
            let result = ReadYakStore::find_yak(&store, "nonexistent");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn read_nonexistent_field_errors() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let result = ReadYakStore::read_field(&store, "test-yak", "nonexistent");
            assert!(result.is_err());
        }

        // --- State & Context via fields ---

        #[test]
        fn state_done_via_field() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            store.write_field("test-yak", STATE_FIELD, "done").unwrap();
            let yak = ReadYakStore::get_yak(&store, "test-yak").unwrap();
            assert!(yak.is_done());
            assert_eq!(yak.state, "done");
        }

        #[test]
        fn context_via_field() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            store
                .write_field("test-yak", CONTEXT_FIELD, "Some context")
                .unwrap();
            let yak = ReadYakStore::get_yak(&store, "test-yak").unwrap();
            assert_eq!(yak.context, Some("Some context".to_string()));
        }

        #[test]
        fn empty_context_is_none() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let yak = ReadYakStore::get_yak(&store, "test-yak").unwrap();
            assert_eq!(yak.context, None);
        }
    };
}

pub(crate) use yak_store_tests;
