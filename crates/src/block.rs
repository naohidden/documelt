use serde::Serialize;

fn is_false(b: &bool) -> bool {
    !*b
}

/// インライン要素。1つの装飾スタイルが続く範囲を表す。
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub text: String,
    #[serde(skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub strike: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub code: bool,
    /// ハイライト色 (OOXML の highlight 値。色名はそのまま渡す)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
    /// ハイパーリンクの URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl Run {
    pub fn text(s: impl Into<String>) -> Self {
        Run {
            text: s.into(),
            ..Default::default()
        }
    }

    /// 装飾のみが一致するか (テキストは見ない)。連続 Run の結合判定に使う。
    pub fn same_style(&self, other: &Run) -> bool {
        self.bold == other.bold
            && self.italic == other.italic
            && self.strike == other.strike
            && self.code == other.code
            && self.highlight == other.highlight
            && self.link == other.link
    }
}

/// ブロック要素。ページ(スライド/シート)は Vec<Block> で表す。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    Heading {
        level: u8,
        runs: Vec<Run>,
    },
    Para {
        runs: Vec<Run>,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    /// rows[行][列] = セル内の Run 列
    Table {
        rows: Vec<Vec<Vec<Run>>>,
    },
    Code {
        text: String,
    },
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ListItem {
    pub level: u8,
    pub runs: Vec<Run>,
}

/// 連続する同スタイルの Run をまとめる (Markdown の記号が細切れになるのを防ぐ)
pub fn merge_runs(runs: Vec<Run>) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for r in runs {
        if r.text.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.same_style(&r) => last.text.push_str(&r.text),
            _ => out.push(r),
        }
    }
    out
}

/// ブロック列からプレーンテキストを復元する (format: 'text' 用)
pub fn blocks_to_text(blocks: &[Block]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for b in blocks {
        match b {
            Block::Heading { runs, .. } | Block::Para { runs } => {
                lines.push(runs_to_text(runs));
            }
            Block::List { items, .. } => {
                for it in items {
                    lines.push(runs_to_text(&it.runs));
                }
            }
            Block::Table { rows } => {
                for row in rows {
                    let cells: Vec<String> = row.iter().map(|c| runs_to_text(c)).collect();
                    lines.push(cells.join("\t"));
                }
            }
            Block::Code { text } => lines.push(text.clone()),
        }
    }
    lines.join("\n").trim().to_string()
}

pub fn runs_to_text(runs: &[Run]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect::<String>()
}
