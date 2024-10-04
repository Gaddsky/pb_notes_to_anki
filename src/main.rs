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

const MODEL_ID: i64 = 1728045059; // the same model should have the same id across all decks

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File path
    file: String,
    /// Deck id. Default value will be generated based on current timestamp
    #[arg(short, long)]
    deck_id: Option<i64>,
    /// Minimum word count
    #[arg(short, long, default_value_t=1)]
    min_count: i32,
}

fn main() {
    let (filepath, book_name, deck_id, min_count) = parse_args().unwrap();
    let filepath = filepath.as_path();

    let collection = parse_html(filepath);
    let pb_notes_deck = create_deck(collection, &book_name, deck_id, min_count);

    pb_notes_deck
        .write_to_file(
            filepath
                .with_extension("apkg")
                .as_os_str()
                .to_str()
                .unwrap(),
        )
        .unwrap();

    let report_content = write_report(filepath, &book_name, deck_id, min_count);
    println!("{}", report_content);
}

fn parse_args() -> Result<(PathBuf, String, i64, i32), &'static str> {
    let args = Args::parse();

    let deck_id = match args.deck_id {
        Some(n) => n,
        None => {
            (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                * 10e8) as i64
        }
    };

    let filepath = PathBuf::from(args.file);
    let book_name = filepath.file_stem().unwrap().to_str().unwrap();

    Ok((filepath.clone(), book_name.to_string(), deck_id, args.min_count))
}

fn parse_html(filepath: &Path) -> HashMap<String, (String, i32)> {
    let html = fs::read_to_string(filepath).unwrap();
    let document = Html::parse_document(&html);
    let bookmark_selector = Selector::parse(".bookmark").unwrap();
    let text_selector = Selector::parse(".bm-text").unwrap();
    let note_selector = Selector::parse(".bm-note").unwrap();

    let mut collection: HashMap<String, (String, i32)> = HashMap::new();

    for bookmark in document.select(&bookmark_selector) {
        if let Some(text) = bookmark.select(&text_selector).next() {
            let word = text.text().collect::<Vec<_>>().join("");

            if let Some(note) = bookmark.select(&note_selector).next() {
                let translation_html = note.inner_html();
                let entry = collection
                    .entry(word.trim().to_string())
                    .or_insert((translation_html.trim().to_string(), 0));
                entry.1 += 1
            }
        }
    }
    collection
}

fn create_deck(
    collection: HashMap<String, (String, i32)>,
    book_name: &str,
    deck_id: i64,
    min_count: i32,
) -> Deck {
    let pb_notes_model = Model::new(
        MODEL_ID,
        "Pocket Book Notes Model",
        vec![Field::new("Word"), Field::new("Translation")],
        vec![
            Template::new("PB Notes card")
                .qfmt(r#"<div class="wordstyle">{{Word}}</div>"#)
                .afmt(r#"{{FrontSide}}<hr id="answer">{{Translation}}"#),
            // TODO reversed card require extracting word transcription
            // Template::new("PB Notes card")
            //     .qfmt(r#"{{Translation}}"#)
            //     .afmt(r#"{{FrontSide}}<hr id="answer"><div class="wordstyle">{{Word}}</div>"#),
        ],
    )
    .css(include_str!("../assets/style.css"));

    let mut pb_notes_deck = Deck::new(
        deck_id,
        &book_name,
        &format!("{book_name}. Deck created from Pocket Book translation notes"),
    );

    for (word, (translation_html, word_count)) in collection {
        if word_count < min_count {
            continue;
        }
        let note = Note::new(
            pb_notes_model.clone(),
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

fn write_report(filepath: &Path, book_name: &str, deck_id: i64, min_count: i32) -> String {
    let report_path = get_report_path(filepath, &book_name);
    let report_content = format!(
        include_str!("../assets/report_template.txt"),
        book_name = book_name,
        deck_id = deck_id,
        filepath = filepath.as_os_str().to_str().unwrap(),
        min_count = min_count
    );

    let mut report_file = File::create(report_path).unwrap();
    report_file.write_all(report_content.as_bytes()).unwrap();

    report_content
}
