//! # PhraseForge: A Passphrase Generator
//!
//! PhraseForge generates easy-to-remember passphrases using words from WordNet.
//! The phrases follow the structure: `adjective-noun-verb-adverb`.
//!
//! ## Features
//! - Downloads and extracts WordNet word lists.
//! - Caches word lists locally for offline use.
//! - Generates passphrases using randomly selected words.
//! - Supports re-downloading word lists with a flag.
//! - Allows generating multiple passphrases at once.
//!
//! ## Usage
//! ```sh
//! phraseforge --count 5   # Generate 5 passphrases
//! phraseforge --redownload  # Force re-download of WordNet data
//! ```
//!
//! ## License
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program. If not, see <https://www.gnu.org/licenses/>.
//!
use directories::ProjectDirs;
use rand::prelude::*;
use reqwest::blocking::get;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;

const WORDNET_URL: &str = "https://wordnetcode.princeton.edu/3.0/WNdb-3.0.tar.gz";
const WORDNET_ARCHIVE: &str = "WNdb-3.0.tar.gz";

fn get_data_dir() -> PathBuf {
    let proj_dirs =
        ProjectDirs::from("com", "tynsol", "phraseforge").expect("Failed to get data directory");
    let data_dir = proj_dirs.data_local_dir().to_path_buf();
    fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    data_dir
}

fn download_and_extract_wordnet(data_dir: &PathBuf, force: bool) {
    let archive_path = data_dir.join(WORDNET_ARCHIVE);
    let dict_dir = data_dir.join("dict");

    if dict_dir.exists() && !force {
        return;
    }

    println!("Downloading WordNet...");
    let response = get(WORDNET_URL).expect("Failed to download WordNet");
    let bytes = response.bytes().expect("Failed to read response bytes");
    fs::write(&archive_path, &bytes).expect("Failed to save archive");

    println!("Extracting WordNet...");
    Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(data_dir)
        .status()
        .expect("Failed to extract WordNet");
}

fn extract_words(wordnet_file: &PathBuf) -> Vec<String> {
    let file = fs::File::open(wordnet_file).expect("Failed to open wordnet file");
    let reader = BufReader::new(file);

    reader
        .lines()
        .filter_map(Result::ok)
        .filter(|line| !line.starts_with("  "))
        .filter_map(|line| line.split_whitespace().next().map(String::from))
        .filter(|word| word.chars().all(char::is_alphabetic)) // Filter out words with underscores, hyphens, numbers, etc.
        .filter(|word| word.len() > 3) // Filter out short words
        .collect()
}

fn save_word_list(words: &[String], file_path: &PathBuf) {
    let mut file = fs::File::create(file_path).expect("Failed to create word list file");
    for word in words {
        writeln!(file, "{}", word).expect("Failed to write word to file");
    }
}

fn load_or_generate_word_lists(data_dir: &PathBuf) {
    let dict_dir = data_dir.join("dict");
    let word_files = vec![
        ("index.adj", "adjectives.txt"),
        ("index.noun", "nouns.txt"),
        ("index.verb", "verbs.txt"),
        ("index.adv", "adverbs.txt"),
    ];

    for (wordnet_file, output_file) in &word_files {
        let wn_path = dict_dir.join(wordnet_file);
        let out_path = data_dir.join(output_file);
        if !out_path.exists() {
            let words = extract_words(&wn_path);
            save_word_list(&words, &out_path);
        }
    }
}

fn load_word_list(file_path: &PathBuf) -> Vec<String> {
    let file = fs::File::open(file_path).expect("Failed to open word list file");
    BufReader::new(file)
        .lines()
        .filter_map(Result::ok)
        .collect()
}

fn generate_password(data_dir: &PathBuf) -> String {
    let adj = load_word_list(&data_dir.join("adjectives.txt"));
    let noun = load_word_list(&data_dir.join("nouns.txt"));
    let verb = load_word_list(&data_dir.join("verbs.txt"));
    let adv = load_word_list(&data_dir.join("adverbs.txt"));

    let mut rng = rand::rng();
    format!(
        "{}-{}-{}-{}",
        adj.as_slice()
            .choose(&mut rng)
            .unwrap_or(&"quick".to_string()),
        noun.as_slice()
            .choose(&mut rng)
            .unwrap_or(&"fox".to_string()),
        verb.as_slice()
            .choose(&mut rng)
            .unwrap_or(&"jumps".to_string()),
        adv.as_slice()
            .choose(&mut rng)
            .unwrap_or(&"swiftly".to_string())
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let force_download = args.contains(&"--redownload".to_string());
    let num_passwords = args
        .iter()
        .position(|arg| arg == "--count")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|num| num.parse::<usize>().ok())
        .unwrap_or(1);

    let data_dir = get_data_dir();
    download_and_extract_wordnet(&data_dir, force_download);
    load_or_generate_word_lists(&data_dir);

    for _ in 0..num_passwords {
        println!("{}", generate_password(&data_dir));
    }
}
