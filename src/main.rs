use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use genanki_rs::{Deck, Field, Model, Note, Template};
use scraper::{Html, Selector};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File path
    file: String,
    /// Model id
    #[arg(short, long)]
    model_id: Option<i64>,
    /// Deck id
    #[arg(short, long)]
    deck_id: Option<i64>,
}

fn main() {
    let (filepath, book_name, deck_id, model_id) = parse_args().unwrap();
    let filepath = filepath.as_path();

    let collection = parse_html(filepath);
    let pb_notes_deck = create_deck(collection, &book_name, deck_id, model_id);

    pb_notes_deck
        .write_to_file(
            filepath
                .with_extension("apkg")
                .as_os_str()
                .to_str()
                .unwrap(),
        )
        .unwrap();

    let report_path = get_report_path(filepath, &book_name);
    write_report(report_path.as_path(), &book_name, deck_id, model_id);

    println!("Anki deck was created with deck_id={deck_id}, model_id={model_id}")
}

fn parse_args() -> Result<(PathBuf, String, i64, i64), &'static str> {
    let args = Args::parse();

    let filepath_string = args.file;
    let model_id = match args.model_id {
        Some(n) => n,
        None => {
            (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                * 10e8) as i64
        }
    };
    let deck_id = match args.deck_id {
        Some(n) => n,
        None => model_id / 100,
    };

    let filepath = Path::new(&filepath_string).to_path_buf();
    let book_name = filepath.file_stem().unwrap().to_str().unwrap();

    Ok((filepath.clone(), book_name.to_string(), deck_id, model_id))
}

fn parse_html(filepath: &Path) -> HashMap<String, String> {
    let html = fs::read_to_string(filepath).unwrap();
    let document = Html::parse_document(&html);
    let bookmark_selector = Selector::parse(".bookmark").unwrap();
    let text_selector = Selector::parse(".bm-text").unwrap();
    let note_selector = Selector::parse(".bm-note").unwrap();

    let mut collection: HashMap<String, String> = HashMap::new();

    for bookmark in document.select(&bookmark_selector) {
        if let Some(text) = bookmark.select(&text_selector).next() {
            let word = text.text().collect::<Vec<_>>().join("");

            if let Some(note) = bookmark.select(&note_selector).next() {
                let translation_html = note.inner_html();
                collection.insert(word.trim().to_string(), translation_html.trim().to_string());
            }
        }
    }
    collection
}

fn create_deck(
    collection: HashMap<String, String>,
    book_name: &str,
    deck_id: i64,
    model_id: i64,
) -> Deck {
    let pb_notes_model = Model::new(
        model_id,
        "Pocket Book Notes Model",
        vec![Field::new("Word"), Field::new("Translation")],
        vec![Template::new("PB Notes card")
            .qfmt(r#"<div class="wordstyle">{{Word}}</div>"#)
            .afmt(r#"{{FrontSide}}<hr id="answer">{{Translation}}"#)],
    )
    .css(include_str!("../assets/style.css"));

    let mut pb_notes_deck = Deck::new(
        deck_id,
        &book_name,
        &format!("{book_name}. Deck created from Pocket Book translation notes"),
    );

    for (word, translation_html) in collection {
        let note = Note::new(
            pb_notes_model.to_owned(),
            vec![&word, &translation_html.trim()],
        )
        .unwrap();
        pb_notes_deck.add_note(note);
    }

    pb_notes_deck
}

fn get_report_path(filepath: &Path, book_name: &str) -> PathBuf {
    let folder = filepath.parent().unwrap().as_os_str().to_str().unwrap();
    let mut report_path = PathBuf::from(folder);
    let report_name = format!("{book_name}_report.txt");
    report_path.push(report_name);
    report_path
}

fn write_report(report_path: &Path, book_name: &str, deck_id: i64, model_id: i64) {
    let report_content = format!(
        include_str!("../assets/report_template.txt"),
        book_name, deck_id, model_id
    );

    let mut report_file = File::create(report_path).unwrap();
    report_file.write_all(report_content.as_bytes()).unwrap();
}
