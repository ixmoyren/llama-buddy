use crate::db;
use rustyline::{
    Cmd, Completer, CompletionType, EditMode, Editor, Helper, Hinter, KeyEvent, Validator,
    completion::FilenameCompleter,
    highlight::{CmdKind, Highlighter, MatchingBracketHighlighter},
    hint::HistoryHinter,
    sqlite_history::SQLiteHistory,
    validate::MatchingBracketValidator,
};
use std::{
    borrow::{
        Cow,
        Cow::{Borrowed, Owned},
    },
    path::Path,
};

pub type ReadLine = Editor<RustyLineHelper, SQLiteHistory>;

pub trait EditorExt {
    fn colored_prompt(&mut self, prompt: impl ToString);
}

impl EditorExt for ReadLine {
    fn colored_prompt(&mut self, prompt: impl ToString) {
        self.helper_mut()
            .expect("Line editor no helper")
            .colored_prompt = prompt.to_string();
    }
}

pub fn new_rustyline(path: impl AsRef<Path>) -> Editor<RustyLineHelper, SQLiteHistory> {
    let config = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Vi)
        .build();
    // 获取 rustyline_history 的数据库
    let db_path =
        db::get_rustyline_history_db_path(&path).expect("Couldn't get rustyline history db path");
    // 历史记录保存在 sqlite 中
    let history =
        SQLiteHistory::open(&config, &db_path).expect("Couldn't open rustyline history db");
    db::change_rustyline_history_scheme(db_path).expect("Couldn't change rustyline history schema");
    // 设置 helper
    let helper = RustyLineHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter::new(),
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };

    let mut line_editor =
        Editor::with_history(config, history).expect("Couldn't create a line editor");
    line_editor.set_helper(Some(helper));
    line_editor.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchBackward);
    line_editor.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchForward);

    line_editor
}

#[derive(Helper, Completer, Validator, Hinter)]
pub struct RustyLineHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Highlighter for RustyLineHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
}
