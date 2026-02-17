/// Contract tests that must pass for all ReadYakStore + WriteYakStore implementations.
/// Use the yak_store_tests! macro to run against any implementation.
///
/// The macro accepts an expression that returns `(impl ReadYakStore + WriteYakStore, _guard)`.
/// The `_guard` keeps any resources (like TempDir) alive for the test duration.
/// For implementations that don't need a guard, pass `()`.
macro_rules! yak_store_tests {
    ($create_store:expr) => {
        use crate::domain::ports::{ReadYakStore, WriteYakStore};
        use crate::domain::slug::YakId;
        use crate::domain::{CONTEXT_FIELD, STATE_FIELD};

        // --- WriteYakStore ---

        #[test]
        fn create_yak_is_retrievable() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
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

            let result = ReadYakStore::get_yak(&store, &YakId::from("old-name"));
            assert!(result.is_err());

            let yak = ReadYakStore::get_yak(&store, &YakId::from("new-name")).unwrap();
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
            let content =
                ReadYakStore::read_field(&store, &YakId::from("test-yak"), "notes").unwrap();
            assert_eq!(content, "Field content");
        }

        #[test]
        fn write_field_with_dots_in_name() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            store
                .write_field("test-yak", "notes.txt", "Text file")
                .unwrap();
            let content =
                ReadYakStore::read_field(&store, &YakId::from("test-yak"), "notes.txt").unwrap();
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
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
            assert_eq!(yak.state, "todo");
            assert_eq!(yak.context, None);
            assert!(!yak.is_done());
        }

        #[test]
        fn get_nonexistent_yak_errors() {
            let (store, _guard) = $create_store;
            let result = ReadYakStore::get_yak(&store, &YakId::from("nonexistent"));
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
        fn fuzzy_find_yak_id_exact_match() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "test-yak").unwrap();
            assert_eq!(result, YakId::from("test-yak"));
        }

        #[test]
        fn fuzzy_find_yak_id_fuzzy_match() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "test").unwrap();
            assert_eq!(result, YakId::from("test-yak"));
        }

        #[test]
        fn fuzzy_find_yak_id_case_insensitive() {
            let (store, _guard) = $create_store;
            store.create_yak("Fix the Bug", "", None).unwrap();
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "the bug").unwrap();
            assert_eq!(result, YakId::from("Fix the Bug"));
        }

        #[test]
        fn fuzzy_find_yak_id_matches_leaf() {
            let (store, _guard) = $create_store;
            store.create_yak("parent", "", None).unwrap();
            store.create_yak("parent/child1", "", None).unwrap();

            let result = ReadYakStore::fuzzy_find_yak_id(&store, "parent").unwrap();
            assert_eq!(result, YakId::from("parent"));

            // Fuzzy search for "child1" should find the child yak
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "child1");
            assert!(result.is_ok(), "Expected to find child1 via fuzzy search");
        }

        #[test]
        fn fuzzy_find_yak_id_leaf_only() {
            let (store, _guard) = $create_store;
            store.create_yak("parent/child1", "", None).unwrap();

            let result = ReadYakStore::fuzzy_find_yak_id(&store, "parent");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn fuzzy_find_yak_id_ambiguous() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak1", "", None).unwrap();
            store.create_yak("test-yak2", "", None).unwrap();
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "test");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("ambiguous"));
        }

        #[test]
        fn fuzzy_find_yak_id_not_found() {
            let (store, _guard) = $create_store;
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "nonexistent");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn read_nonexistent_field_errors() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let result = ReadYakStore::read_field(&store, &YakId::from("test-yak"), "nonexistent");
            assert!(result.is_err());
        }

        // --- State & Context via fields ---

        #[test]
        fn state_done_via_field() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            store.write_field("test-yak", STATE_FIELD, "done").unwrap();
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
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
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
            assert_eq!(yak.context, Some("Some context".to_string()));
        }

        #[test]
        fn empty_context_is_none() {
            let (store, _guard) = $create_store;
            store.create_yak("test-yak", "", None).unwrap();
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
            assert_eq!(yak.context, None);
        }
    };
}

pub(crate) use yak_store_tests;
