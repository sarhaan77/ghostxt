use ghostxt::{Action, Editor};

#[test]
fn applies_a_basic_editing_flow() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.txt");
    std::fs::write(&path, "hello world").unwrap();

    let mut editor = Editor::open(&path).unwrap();
    editor.apply(Action::MoveFileEnd, 20, 10).unwrap();
    editor.apply(Action::Newline, 20, 10).unwrap();
    editor.apply(Action::Insert("next".into()), 20, 10).unwrap();
    editor.apply(Action::DeleteLine, 20, 10).unwrap();
    editor.apply(Action::Insert("line".into()), 20, 10).unwrap();
    editor.apply(Action::Save, 20, 10).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world\nline");
}
