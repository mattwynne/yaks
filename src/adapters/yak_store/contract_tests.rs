/// Contract tests that must pass for all ReadYakStore + WriteYakStore implementations.
/// Use the yak_store_tests! macro to run against any implementation.
///
/// The macro accepts an expression that returns `(impl ReadYakStore + WriteYakStore, _guard)`.
/// The `_guard` keeps any resources (like TempDir) alive for the test duration.
/// For implementations that don't need a guard, pass `()`.
macro_rules! yak_store_tests {
    ($create_store:expr) => {
        use crate::domain::ports::{ReadYakStore, WriteYakStore};
        use crate::domain::slug::{Name, YakId};
        use crate::domain::{CONTEXT_FIELD, STATE_FIELD};

        // --- WriteYakStore ---

        #[test]
        fn create_yak_is_retrievable() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
            assert_eq!(yak.name, "test-yak");
        }

        #[test]
        fn create_duplicate_yak_errors() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            let result = store.create_yak(&Name::from("test-yak"), &YakId::from(""), None);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("already exists"));
        }

        #[test]
        fn delete_yak_removes_it() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            store.delete_yak(&YakId::from("test-yak")).unwrap();
            assert!(ReadYakStore::get_yak(&store, &YakId::from("test-yak")).is_err());
        }

        #[test]
        fn delete_nonexistent_yak_succeeds() {
            let (store, _guard) = $create_store;
            let result = store.delete_yak(&YakId::from("nonexistent"));
            assert!(result.is_ok());
        }

        #[test]
        fn rename_yak_moves_with_fields() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("old-name"), &YakId::from(""), None)
                .unwrap();
            store
                .write_field(&YakId::from("old-name"), CONTEXT_FIELD, "Context text")
                .unwrap();
            store
                .write_field(&YakId::from("old-name"), STATE_FIELD, "done")
                .unwrap();

            store
                .rename_yak(&YakId::from("old-name"), &Name::from("new-name"))
                .unwrap();

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
            let result = store.rename_yak(&YakId::from("nonexistent"), &Name::from("new-name"));
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn rename_to_existing_yak_errors() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("yak1"), &YakId::from(""), None)
                .unwrap();
            store
                .create_yak(&Name::from("yak2"), &YakId::from(""), None)
                .unwrap();
            let result = store.rename_yak(&YakId::from("yak1"), &Name::from("yak2"));
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("already exists"));
        }

        #[test]
        fn write_field_is_readable() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            store
                .write_field(&YakId::from("test-yak"), "notes", "Field content")
                .unwrap();
            let content =
                ReadYakStore::read_field(&store, &YakId::from("test-yak"), "notes").unwrap();
            assert_eq!(content, "Field content");
        }

        #[test]
        fn write_field_with_dots_in_name() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            store
                .write_field(&YakId::from("test-yak"), "notes.txt", "Text file")
                .unwrap();
            let content =
                ReadYakStore::read_field(&store, &YakId::from("test-yak"), "notes.txt").unwrap();
            assert_eq!(content, "Text file");
        }

        #[test]
        fn write_field_nonexistent_yak_errors() {
            let (store, _guard) = $create_store;
            let result = store.write_field(&YakId::from("nonexistent"), "field", "content");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        // --- ReadYakStore ---

        #[test]
        fn get_yak_defaults() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
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
            store
                .create_yak(&Name::from("yak1"), &YakId::from(""), None)
                .unwrap();
            store
                .create_yak(&Name::from("yak2"), &YakId::from(""), None)
                .unwrap();
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
        fn fuzzy_find_yak_id_exact_match() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "test-yak").unwrap();
            assert_eq!(result, YakId::from("test-yak"));
        }

        #[test]
        fn fuzzy_find_yak_id_fuzzy_match() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "test").unwrap();
            assert_eq!(result, YakId::from("test-yak"));
        }

        #[test]
        fn fuzzy_find_yak_id_case_insensitive() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("Fix the Bug"), &YakId::from(""), None)
                .unwrap();
            let result = ReadYakStore::fuzzy_find_yak_id(&store, "the bug").unwrap();
            assert_eq!(result, YakId::from("Fix the Bug"));
        }

        #[test]
        fn fuzzy_find_yak_id_matches_leaf() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("parent"), &YakId::from(""), None)
                .unwrap();
            store
                .create_yak(&Name::from("parent/child1"), &YakId::from(""), None)
                .unwrap();

            let result = ReadYakStore::fuzzy_find_yak_id(&store, "parent").unwrap();
            assert_eq!(result, YakId::from("parent"));

            // Fuzzy search for "child1" should find the child yak
            // and the returned ID should resolve to the correct yak
            let child_id = ReadYakStore::fuzzy_find_yak_id(&store, "child1").unwrap();
            let child = ReadYakStore::get_yak(&store, &child_id).unwrap();
            assert_eq!(child.name, "parent/child1");
        }

        #[test]
        fn fuzzy_find_yak_id_leaf_only() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("parent/child1"), &YakId::from(""), None)
                .unwrap();

            let result = ReadYakStore::fuzzy_find_yak_id(&store, "parent");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn fuzzy_find_yak_id_ambiguous() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak1"), &YakId::from(""), None)
                .unwrap();
            store
                .create_yak(&Name::from("test-yak2"), &YakId::from(""), None)
                .unwrap();
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
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            let result = ReadYakStore::read_field(&store, &YakId::from("test-yak"), "nonexistent");
            assert!(result.is_err());
        }

        // --- State & Context via fields ---

        #[test]
        fn state_done_via_field() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            store
                .write_field(&YakId::from("test-yak"), STATE_FIELD, "done")
                .unwrap();
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
            assert!(yak.is_done());
            assert_eq!(yak.state, "done");
        }

        #[test]
        fn context_via_field() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            store
                .write_field(&YakId::from("test-yak"), CONTEXT_FIELD, "Some context")
                .unwrap();
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
            assert_eq!(yak.context, Some("Some context".to_string()));
        }

        #[test]
        fn empty_context_is_none() {
            let (store, _guard) = $create_store;
            store
                .create_yak(&Name::from("test-yak"), &YakId::from(""), None)
                .unwrap();
            let yak = ReadYakStore::get_yak(&store, &YakId::from("test-yak")).unwrap();
            assert_eq!(yak.context, None);
        }
    };
}

pub(crate) use yak_store_tests;
