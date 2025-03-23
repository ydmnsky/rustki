use rand::seq::SliceRandom;
use rusqlite::{params, Connection, Result};
use std::env;
use std::io::{self, Write};
use std::error::Error;
use std::fs;
use dirs::config_dir;

#[derive(Debug, Clone)]
struct Word {
    word: String,
    translation: String,
    knowledge: i32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let conn = Connection::open(get_db_path()?)?;

    create_table(&conn)?;

    if args.len() > 1 {
        match args[1].as_str() {
            "add" if args.len() == 4 => {
                add_word(&conn, &args[2], &args[3])?;
                println!("Добавлено слово: {} -> {}", args[2], args[3]);
            }
            "remove" if args.len() == 3 => {
                remove_word(&conn, &args[2])?;
                println!("Удалено слово: {}", args[2]);
            }
            "clear" => {
                clear_database(&conn)?;
                println!("Словарь очищен.");
            }
            _ => {
                eprintln!("Usage: rustki [add <word> <translation> | remove <word> | clear]");
            }
        }
    } else {
        run_trainer(&conn)?;
    }

    Ok(())
}

fn get_db_path() -> Result<String, Box<dyn Error>> {
    if let Ok(path) = env::var("RUSTKI_DB_PATH") {
        return Ok(path);
    }

    let mut default_path = config_dir().unwrap_or_else(|| ".".into());
    default_path.push("rustki");
    fs::create_dir_all(&default_path)?;
    default_path.push("words.db");

    Ok(default_path.to_str().unwrap().to_string())
}

fn create_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS words (
            word TEXT PRIMARY KEY,
            translation TEXT NOT NULL,
            knowledge INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn add_word(conn: &Connection, word: &str, translation: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO words (word, translation, knowledge) VALUES (?1, ?2, 0)",
        params![word, translation],
    )?;
    Ok(())
}

fn run_trainer(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let words = get_words_for_training(conn)?;
    if words.len() < 10 {
        println!("You need to add more words to the dictionary (at least 10). You have {}", words.len());
        return Ok(());
    }

    let mut score = 0;
    let mut max_score = 0;

    for word in words {
        if exercise(conn, &word)? {
            score += 1;
        };
        max_score += 1;
    }

    print!("TOTAL SCORE: {}/{}", score, max_score);
    
    Ok(())
}

fn get_words_for_training(conn: &Connection) -> Result<Vec<Word>> {
    let mut stmt = conn.prepare("SELECT word, translation, knowledge FROM words")?;
    let words_iter = stmt.query_map([], |row| {
        Ok(Word {
            word: row.get(0)?,
            translation: row.get(1)?,
            knowledge: row.get(2)?,
        })
    })?;

    let words: Vec<Word> = words_iter.filter_map(|x| x.ok()).collect();

    let mut easy_words: Vec<Word> = words.iter().filter(|w| w.knowledge == 4).cloned().collect();
    let mut other_words: Vec<Word> = words.iter().filter(|w| w.knowledge >= 0 && w.knowledge <= 3).cloned().collect();

    other_words.shuffle(&mut rand::thread_rng());
    easy_words.shuffle(&mut rand::thread_rng());

    let mut selected_words = Vec::new();

    selected_words.extend(other_words.into_iter().take(8));

    selected_words.extend(easy_words.into_iter().take(2));

    if selected_words.len() < 10 {
        let remaining_count = 10 - selected_words.len();
        let mut all_words = words.clone();
        all_words.shuffle(&mut rand::thread_rng());
        selected_words.extend(all_words.into_iter().take(remaining_count));
    }

    selected_words.truncate(10);

    Ok(selected_words)
}


fn exercise(conn: &Connection, word: &Word) -> Result<bool, Box<dyn Error>> {

    let correct = match word.knowledge {
        0 => multiple_choice(conn, word, true)?,
        1 => multiple_choice(conn, word, false)?,
        2 => written_answer(word, false)?,
        3 | 4 => written_answer(word, true)?,
        _ => false,
    };

    let correctness: bool;

    let new_knowledge = if correct {
        println!("Correct!\n");
        correctness = true;
        (word.knowledge + 1).min(4)
    } else {
        println!("Wrong!");
        correctness = false;
        println!("Cлово {} — переводтся как {}\n", word.word, word.translation);
        (word.knowledge - 1).max(0)
    };

    conn.execute(
        "UPDATE words SET knowledge = ?1 WHERE word = ?2",
        params![new_knowledge, word.word],
    )?;

    Ok(correctness)
}

fn multiple_choice(conn: &Connection, word: &Word, word_to_translation: bool) -> Result<bool, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT word, translation FROM words WHERE word != ?1")?;
    let mut options: Vec<(String, String)> = stmt
        .query_map(params![word.word], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|x| x.ok())
        .collect();

    if word_to_translation {
        println!("Выберите перевод на русский: {}", word.word);
    } else {
        println!("Выберите перевод на сербский: {}", word.translation);
    }

    options.shuffle(&mut rand::thread_rng());
    options.truncate(3);

    let correct_option = if word_to_translation {
        word.translation.clone()
    } else {
        word.word.clone()
    };

    let mut display_options: Vec<String> = options
        .iter()
        .map(|(w, t)| if word_to_translation { t.clone() } else { w.clone() })
        .collect();

    display_options.push(correct_option.clone());
    display_options.shuffle(&mut rand::thread_rng());

    for (i, display) in display_options.iter().enumerate() {
        println!("{}: {}", i + 1, display);
    }

    print!("Your answer: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim().parse::<usize>().unwrap_or(0);

    Ok(choice > 0 && choice <= display_options.len() && display_options[choice - 1] == correct_option)
}


fn written_answer(word: &Word, reverse: bool) -> Result<bool, Box<dyn Error>> {
    let prompt = if reverse {
        format!("Переведите на сербский: {}", word.translation)
    } else {
        format!("Переведите на русский: {}", word.word)
    };

    println!("{}", prompt);
    print!("Your answer: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let answer = input.trim();

    Ok(if reverse {
        answer == word.word
    } else {
        answer == word.translation
    })
}

fn remove_word(conn: &Connection, word: &str) -> Result<()> {
    let rows_deleted = conn.execute("DELETE FROM words WHERE word = ?1", params![word])?;
    if rows_deleted == 0 {
        println!("Word '{}' not found in the database.", word);
    }
    Ok(())
}

fn clear_database(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM words", [])?;
    Ok(())
}
